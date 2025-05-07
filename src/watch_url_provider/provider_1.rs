use super::WatchURLProvider;

pub fn new(watch_url_template: String) -> impl WatchURLProvider {
    Provider { watch_url_template }
}
pub struct Provider {
    watch_url_template: String,
}

impl WatchURLProvider for Provider {
    fn build_url(&self, episode: &crate::application::Episode) -> String {
        self.watch_url_template
            .replace("{season}", &episode.season().to_string())
    }
}
