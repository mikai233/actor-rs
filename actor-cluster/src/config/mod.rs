use serde::{Deserialize, Serialize};
use actor_core::config::Config;
use actor_remote::config::RemoteConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub remote: RemoteConfig,
}

impl Config for ClusterConfig {
    fn merge(&self, other: Self) -> Self {
        let ClusterConfig { remote } = other;
        Self {
            remote,
        }
    }
}
