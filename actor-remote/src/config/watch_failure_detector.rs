use imstr::ImString;
use serde::{Deserialize, Serialize};

use actor_core::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFailureDetector {
    #[serde(rename = "implementation-class")]
    pub implementation_class: ImString,
    #[serde(rename = "heartbeat-interval")]
    pub heartbeat_interval: ConfigDuration,
    pub threshold: f64,
    #[serde(rename = "max-sample-size")]
    pub max_sample_size: usize,
    #[serde(rename = "min-std-deviation")]
    pub min_std_deviation: ConfigDuration,
    #[serde(rename = "acceptable-heartbeat-pause")]
    pub acceptable_heartbeat_pause: ConfigDuration,
    #[serde(rename = "unreachable-nodes-reaper-interval")]
    pub unreachable_nodes_reaper_interval: ConfigDuration,
    #[serde(rename = "expected-response-after")]
    pub expected_response_after: ConfigDuration,
}