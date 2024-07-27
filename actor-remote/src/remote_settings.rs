use std::net::SocketAddr;
use std::time::Duration;

use config::Config;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use actor_core::message::message_registry::MessageRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSettings {
    pub warn_about_direct_use: bool,
    pub use_unsafe_remote_features_outside_cluster: bool,
    pub warn_unsafe_watch_outside_cluster: bool,
    pub watch_failure_detector: WatchFailureDetector,
    pub deployment: Deployment,
    pub artery: Artery,
    pub advanced: Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFailureDetector {
    pub implementation_class: ImString,
    pub heartbeat_interval: Duration,
    pub threshold: f64,
    pub max_sample_size: usize,
    pub min_std_deviation: Duration,
    pub acceptable_heartbeat_pause: Duration,
    pub unreachable_nodes_reaper_interval: Duration,
    pub expected_response_after: Duration,
}

impl RemoteSettings {
    pub fn new(config: &Config, reg: MessageRegistry) -> Self {
        unimplemented!("TODO")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub enable_allow_list: bool,
    pub allowed_actor_classes: Vec<ImString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artery {
    pub enabled: bool,
    pub transport: ImString,
    pub canonical: SocketAddr,
    pub untrusted_mode: bool,
    pub trusted_selection_paths: Vec<ImString>,
    pub log_received_messages: bool,
    pub log_sent_messages: bool,
    pub log_frame_size_exceeding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advanced {
    pub maximum_frame_size: usize,
    pub outbound_message_queue_size: usize,
    pub outbound_control_queue_size: usize,
    pub outbound_large_message_queue_size: usize,
    pub system_message_buffer_size: usize,
}
