use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingletonProxyConfig {
    pub singleton_name: String,
    pub role: Option<String>,
    pub singleton_identification_interval: Duration,
    pub buffer_size: usize,
}