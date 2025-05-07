use friends_random_bot_rust::{application, bot, config};
use log::LevelFilter;
use std::{path::Path, sync::Arc};

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_env(env_logger::DEFAULT_FILTER_ENV)
        .init();

    log::info!("Reading config...");
    let config = match config::new(Path::new("config.json")) {
        Ok(config) => config,
        Err(err) => {
            log::error!("{err}");
            return;
        }
    };

    let application = Arc::new(application::new());

    log::info!("Starting bot...");
    bot::new(config.bot_token, application)
        .await
        .dispatch()
        .await;
}
