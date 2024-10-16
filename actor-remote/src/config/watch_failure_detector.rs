use actor_core::config::duration::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFailureDetector {
    pub implementation_class: String,
    pub heartbeat_interval: Duration,
    pub threshold: f64,
    pub max_sample_size: usize,
    pub min_std_deviation: Duration,
    pub acceptable_heartbeat_pause: Duration,
    pub unreachable_nodes_reaper_interval: Duration,
    pub expected_response_after: Duration,
}
