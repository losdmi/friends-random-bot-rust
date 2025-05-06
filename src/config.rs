use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bot_token: String,
}

pub fn new(path: &Path) -> Result<Config, config::ConfigError> {
    let path_str = match path.to_str() {
        Some(str) => str,
        None => {
            return Err(config::ConfigError::Message(String::from(
                "cannot parse path parameter",
            )));
        }
    };

    config::Config::builder()
        .add_source(config::File::with_name(path_str).required(true))
        .build()?
        .try_deserialize()
}
