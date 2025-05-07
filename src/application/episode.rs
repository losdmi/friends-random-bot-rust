#[derive(PartialEq, Debug, Eq, Hash)]
pub struct Episode {
    code: String,
    season: u8,
    episode: u8,
}

impl Episode {
    pub fn from(code: &str) -> Self {
        assert!(code.len() == 6);
        let chars: Vec<char> = code.chars().collect();
        assert!(chars[0] == 's');
        assert!(chars[3] == 'e');

        Self {
            code: code.to_string(),
            season: code[1..=2].parse().expect("cant parse season"),
            episode: code[4..=5].parse().expect("cant parse episode"),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn season(&self) -> u8 {
        self.season
    }

    pub fn episode(&self) -> u8 {
        self.episode
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn episode_from_fn_works_as_expected() {
        let code = "s01e03".to_string();

        let episode = Episode::from(&code);

        assert_eq!(
            episode,
            Episode {
                code,
                season: 1,
                episode: 3
            }
        );
    }
}
