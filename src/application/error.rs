#[derive(Debug)]
pub enum Error {
    NoUnseenEpisodes,
    FileError(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_string = match self {
            Error::NoUnseenEpisodes => "Не осталось непросмотренных эпизодов".to_string(),
            Error::FileError(error) => {
                format!("Ошибка при работе с файлами: {error}")
            }
        };

        write!(f, "{}", as_string)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::FileError(err)
    }
}
