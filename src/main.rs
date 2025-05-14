use friends_random_bot_rust::{application, bot, config, watch_url_provider};
use std::{path::Path, sync::Arc};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info")) // Fallback level
                .expect("error while setting up EnvFilter"),
        )
        .with_target(false)
        .json()
        .flatten_event(true)
        .init();

    tracing::info!("Reading config...");
    let config = match config::new(Path::new("config.json")) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("{err}");
            return;
        }
    };

    let application = Arc::new(application::new(config.storage_path));
    let watch_url_provider = Arc::new(watch_url_provider::provider_1::new(
        config.watch_url_template,
    ));

    tracing::info!("Starting bot...");
    bot::new(config.bot_token, application, watch_url_provider)
        .await
        .dispatch()
        .await;
}
