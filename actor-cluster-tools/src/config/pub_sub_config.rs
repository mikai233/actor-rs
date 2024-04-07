use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubConfig {
    pub name: String,
    pub role: Option<String>,
    pub routing_logic: RoutingLogic,
    pub removed_time_to_live: Duration,
    pub max_delta_elements: usize,
    pub send_to_dead_letters_when_no_subscribers: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum RoutingLogic {
    Random,
    RoundRobin,
    Broadcast,
}