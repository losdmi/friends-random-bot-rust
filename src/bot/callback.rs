use super::error::Error;

pub enum Command {
    MarkSeen(String),
    ClearSeenEpisodes(ClearSeenEpisodesOption),
}

impl Command {
    fn from(command: &str, parameter: &str) -> Result<Command, Error> {
        match command {
            "mark_seen" => Ok(Command::MarkSeen(parameter.to_string())),
            "clear_seen_episodes" => {
                ClearSeenEpisodesOption::from(parameter).map(Command::ClearSeenEpisodes)
            }
            _ => Err(Error::CallbackCommandParseError(format!(
                "неопознанная команда: command={command}"
            ))),
        }
    }

    pub fn from_data_string(data: &str) -> Result<Command, Error> {
        let splitted: Vec<&str> = data.split("=").collect();

        if splitted.len() != 2 {
            // ожидаем что в data лежит строка вида `<command>=<parameter>`
            return Err(Error::CallbackCommandParseError(format!(
                "почему-то в data не то что ожидали: data={data}"
            )));
        }

        Self::from(splitted[0], splitted[1])
    }
}

pub enum ClearSeenEpisodesOption {
    No,
    Yes,
}

impl ClearSeenEpisodesOption {
    fn from(option: &str) -> Result<ClearSeenEpisodesOption, Error> {
        match option.to_lowercase().as_str() {
            "no" => Ok(ClearSeenEpisodesOption::No),
            "yes" => Ok(ClearSeenEpisodesOption::Yes),
            _ => Err(Error::CallbackCommandParseError(format!(
                "неопознанный вариант для команды ClearSeenEpisodes: option={option}"
            ))),
        }
    }
}
