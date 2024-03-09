use std::time::Duration;

use serde::{Deserialize, Serialize};

use actor_cluster_tools::singleton::cluster_singleton_manager::ClusterSingletonManagerSettings;
use actor_core::config::Config;
use actor_derive::AsAny;

mod passivation;

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsAny)]
pub struct ClusterShardingConfig {
    #[serde(default = "default_guardian_name")]
    pub guardian_name: String,
    pub role: Option<String>,
    pub coordinator_singleton_settings: ClusterSingletonManagerSettings,
    pub shard_region_query_timeout: Duration,
    pub retry_interval: Duration,
    pub handoff_timeout: Duration,
}

impl Config for ClusterShardingConfig {
    fn with_fallback(&self, other: Self) -> Self where Self: Sized {
        todo!()
    }
}

fn default_guardian_name() -> String {
    "sharing".to_string()
}