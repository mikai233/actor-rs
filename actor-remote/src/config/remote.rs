use serde::{Deserialize, Serialize};

use crate::config::advanced::Advanced;
use crate::config::artery::Artery;
use crate::config::deployment::Deployment;
use crate::config::watch_failure_detector::WatchFailureDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remote {
    pub warn_about_direct_use: bool,
    pub use_unsafe_remote_features_outside_cluster: bool,
    pub warn_unsafe_watch_outside_cluster: bool,
    pub watch_failure_detector: WatchFailureDetector,
    pub deployment: Deployment,
    pub artery: Artery,
    pub advanced: Advanced,
}
