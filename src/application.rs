mod episode;
mod episodes;
pub mod error;

pub use episode::Episode;
use episodes::EPISODES;
use error::Error;
use rand::seq::IndexedRandom;
use std::fs;
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn new(storage_path: PathBuf) -> Application {
    Application { storage_path }
}

pub struct UserID(u64);

impl UserID {
    pub fn new(user_id: u64) -> Self {
        Self(user_id)
    }
}

impl Display for UserID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Application {
    storage_path: PathBuf,
}

impl Application {
    pub fn get_next_episode(&self, user_id: UserID) -> Result<Episode, Error> {
        let user_storage_path = self.build_user_storage_path(user_id);
        let seen_episodes = self.read_db_from_file(&user_storage_path)?;
        let selected_episode = self.select_next_episode(&seen_episodes)?;

        Ok(selected_episode)
    }

    fn build_user_storage_path(&self, user_id: UserID) -> PathBuf {
        self.storage_path.join(format!("{user_id}.txt"))
    }

    fn read_db_from_file(&self, path: &Path) -> Result<Vec<Episode>, std::io::Error> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                _ => return Err(err),
            },
        };

        let mut seen_episodes = String::new();
        file.read_to_string(&mut seen_episodes)?;

        Ok(seen_episodes
            .trim()
            .lines()
            .rev()
            .map(Episode::from)
            .collect())
    }

    fn select_next_episode(&self, seen_episodes: &[Episode]) -> Result<Episode, Error> {
        let seen_set: std::collections::HashSet<&Episode> = seen_episodes.iter().collect();

        let next_episode = EPISODES
            .choose_multiple(&mut rand::rng(), EPISODES.len())
            .map(|&s| Episode::from(s))
            .find(|ep| !seen_set.contains(ep));

        match next_episode {
            Some(episode) => Ok(episode),
            None => Err(Error::NoUnseenEpisodes),
        }
    }

    pub fn mark_seen(&self, user_id: UserID, episode: Episode) -> Result<(), Error> {
        let user_storage_path = self.build_user_storage_path(user_id);
        let mut seen_episodes = self.read_db_from_file(&user_storage_path)?;

        seen_episodes.push(episode);

        self.save_db_to_file(seen_episodes, &user_storage_path)?;

        Ok(())
    }

    fn save_db_to_file(
        &self,
        seen_episodes: Vec<Episode>,
        path: &Path,
    ) -> Result<(), std::io::Error> {
        if seen_episodes.is_empty() {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            self.create_directory_if_not_exists(parent)?;
        }

        let mut file = File::create(path)?;

        write!(
            file,
            "{}",
            seen_episodes
                .iter()
                .fold(String::new(), |acc, ep| format!("{}\n{}", ep.code(), acc))
        )?;

        Ok(())
    }

    fn create_directory_if_not_exists(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(path)?;

        Ok(())
    }

    pub fn list_seen_episodes(&self, user_id: UserID) -> Result<Vec<Episode>, Error> {
        let user_storage_path = self.build_user_storage_path(user_id);
        let seen_episodes = self.read_db_from_file(&user_storage_path)?;

        Ok(seen_episodes)
    }
}

#[cfg(test)]
mod test {
    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    fn build_application() -> Application {
        Application {
            storage_path: "seen_episodes".into(),
        }
    }

    #[test]
    fn application_build_user_storage_path_fn_works_as_expected() {
        let a = build_application();

        let user_id = UserID(317);

        let result = a.build_user_storage_path(user_id);

        assert_eq!(result, PathBuf::from("seen_episodes/317.txt"));
    }

    #[test]
    fn application_read_db_from_file_fn_works_with_non_existing_file() {
        let a = build_application();

        let result = a.read_db_from_file(Path::new("non_existing_file.txt"));

        assert!(!result.is_err(), "result is error: {result:#?}");
        assert_eq!(result.unwrap(), Vec::<Episode>::new());
    }

    #[test]
    fn application_read_db_from_file_fn_handles_empty_file() {
        let a = build_application();
        let tmpfile = NamedTempFile::new().unwrap();

        let result = a.read_db_from_file(tmpfile.path());
        assert!(!result.is_err(), "result is error: {result:#?}");
        assert_eq!(result.unwrap(), Vec::<Episode>::new());
    }

    #[test]
    fn application_read_db_from_file_fn_reads_data_from_file() {
        let a = build_application();
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "s01e02\ns01e01\n").unwrap();

        let result = a.read_db_from_file(tmpfile.path());
        assert!(!result.is_err(), "result is error: {result:#?}");
        assert_eq!(
            result.unwrap(),
            vec!(Episode::from("s01e01"), Episode::from("s01e02"),)
        );
    }

    #[test]
    fn application_select_next_episode_fn_returns_any_episode_at_all() {
        let a = build_application();

        let result = a.select_next_episode(&Vec::new());
        assert!(!result.is_err(), "result is error: {result:#?}");
        assert!(EPISODES.contains(&result.unwrap().code()))
    }

    #[test]
    fn application_select_next_episode_fn_returns_error_if_there_is_no_unseen_episodes() {
        let a = build_application();

        let all_episodes: Vec<Episode> = EPISODES.iter().map(|s| Episode::from(s)).collect();

        let result = a.select_next_episode(&all_episodes);

        assert!(matches!(result, Err(Error::NoUnseenEpisodes)));
    }

    #[test]
    fn application_save_db_to_file_fn_saves_empty_list_to_file() {
        let a = build_application();

        let mut tmpfile = NamedTempFile::new().unwrap();

        let result = a.save_db_to_file(Vec::new(), &tmpfile.path());
        assert!(!result.is_err(), "result is error: {result:#?}");

        let mut tmpfile_content = String::new();
        tmpfile.read_to_string(&mut tmpfile_content).unwrap();
        assert_eq!(tmpfile_content, "");
    }

    #[test]
    fn application_save_db_to_file_fn_saves_non_empty_list_to_file_in_reverse_order() {
        let a = build_application();

        let mut tmpfile = NamedTempFile::new().unwrap();

        let seen_episodes = vec![Episode::from("s01e01"), Episode::from("s01e02")];

        let result = a.save_db_to_file(seen_episodes, &tmpfile.path());
        assert!(!result.is_err(), "result is error: {result:#?}");

        let mut tmpfile_content = String::new();
        tmpfile.read_to_string(&mut tmpfile_content).unwrap();
        assert_eq!(tmpfile_content, "s01e02\ns01e01\n");
    }

    #[test]
    fn application_create_directory_if_not_exists_fn_creates_multiple_directories() {
        let a = build_application();

        let tmpdir = TempDir::new().unwrap();

        let path = tmpdir.path().join("some/path");
        eprintln!("{}", path.to_string_lossy());
        assert!(!path.exists());

        let result = a.create_directory_if_not_exists(&path);
        assert!(!result.is_err(), "result is error: {result:#?}");
        assert!(path.exists());
    }

    #[test]
    fn application_create_directory_if_not_exists_fn_does_not_error_if_directories_already_exists()
    {
        let a = build_application();

        // Setup temp directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_path = temp_dir.path().join("existing").join("subdirectory");

        // First create the directory structure
        fs::create_dir_all(&test_path).expect("Failed to create initial directory structure");

        // Test the function - should not error even though dir exists
        let result = a.create_directory_if_not_exists(&test_path);

        // Verify
        assert!(
            result.is_ok(),
            "Function should not error when directory already exists"
        );

        // Additional verification that directory still exists
        assert!(
            test_path.exists(),
            "Directory should still exist after function call"
        );
        assert!(
            test_path.is_dir(),
            "Path should still be a directory after function call"
        );
    }
}
