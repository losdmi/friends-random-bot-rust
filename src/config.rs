use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bot_token: String,
    pub storage_path: PathBuf,
    pub watch_url_template: String,
}

pub fn new(config_path: &Path) -> Result<Config, config::ConfigError> {
    let path_str = match config_path.to_str() {
        Some(str) => str,
        None => {
            return Err(config::ConfigError::Message(String::from(
                "cannot parse config_path parameter",
            )));
        }
    };

    config::Config::builder()
        .add_source(config::File::with_name(path_str).required(true))
        .build()?
        .try_deserialize()
}
