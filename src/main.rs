use friends_random_bot_rust::{bot, config};
use log::LevelFilter;
use std::path::Path;

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

    log::info!("Starting bot...");
    bot::new(config.bot_token).await.dispatch().await;
}
