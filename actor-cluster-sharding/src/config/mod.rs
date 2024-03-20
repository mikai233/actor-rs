use std::time::Duration;

use serde::{Deserialize, Serialize};

use actor_cluster_tools::singleton::cluster_singleton_manager::ClusterSingletonManagerSettings;
use actor_core::config::Config;
use actor_derive::AsAny;

mod passivation;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterShardingConfig {
    #[serde(default = "default_guardian_name")]
    pub guardian_name: String,
    pub role: Option<String>,
    pub coordinator_singleton_settings: ClusterSingletonManagerSettings,
    pub shard_region_query_timeout: Duration,
    pub retry_interval: Duration,
    pub handoff_timeout: Duration,
    pub shard_start_timeout: Duration,
    // pub min_nr_or_members: usize,
    pub rebalance_interval: Duration,
}

impl Config for ClusterShardingConfig {
    fn with_fallback(&self, other: Self) -> Self where Self: Sized {
        todo!()
    }
}

fn default_guardian_name() -> String {
    "sharding".to_string()
}

impl Default for ClusterShardingConfig {
    fn default() -> Self {
        Self {
            guardian_name: default_guardian_name(),
            role: None,
            coordinator_singleton_settings: Default::default(),
            shard_region_query_timeout: Duration::from_secs(3),
            retry_interval: Duration::from_millis(200),
            handoff_timeout: Duration::from_secs(60),
            shard_start_timeout: Duration::from_secs(10),
            rebalance_interval: Duration::from_secs(10),
        }
    }
}