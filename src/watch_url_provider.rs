use crate::application::Episode;

pub mod provider_1;

pub trait WatchURLProvider {
    fn build_url(&self, episode: &Episode) -> String;
}
