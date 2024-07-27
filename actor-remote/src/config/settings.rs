use config::Config;
use serde::{Deserialize, Serialize};

use crate::config::advanced::Advanced;
use crate::config::artery::Artery;
use crate::config::deployment::Deployment;
use crate::config::watch_failure_detector::WatchFailureDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "warn-about-direct-use")]
    pub warn_about_direct_use: bool,
    #[serde(rename = "use-unsafe-remote-features-outside-cluster")]
    pub use_unsafe_remote_features_outside_cluster: bool,
    #[serde(rename = "warn-unsafe-watch-outside-cluster")]
    pub warn_unsafe_watch_outside_cluster: bool,
    #[serde(rename = "watch-failure-detector")]
    pub watch_failure_detector: WatchFailureDetector,
    pub deployment: Deployment,
    pub artery: Artery,
    pub advanced: Advanced,
}


impl Settings {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let settings: Self = config.get("akka.remote")?;
        Ok(settings)
    }
}