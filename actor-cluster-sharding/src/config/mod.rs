use std::time::Duration;

use config::{File, FileFormat, Source};
use config::builder::DefaultState;
use serde::{Deserialize, Serialize};

use actor_cluster_tools::config::singleton_config::SingletonConfig;
use actor_core::AsAny;
use actor_core::config::{Config, ConfigBuilder};

use crate::CLUSTER_SHARDING_CONFIG;
use crate::config::passivation::Passivation;

pub mod passivation;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterShardingConfig {
    pub guardian_name: String,
    pub role: Option<String>,
    pub passivation: Passivation,
    pub coordinator_failure_backoff: Duration,
    pub retry_interval: Duration,
    pub buffer_size: usize,
    pub handoff_timeout: Duration,
    pub shard_start_timeout: Duration,
    pub rebalance_interval: Duration,
    pub shard_region_query_timeout: Duration,
    pub coordinator_singleton: SingletonConfig,
    pub coordinator_singleton_role_override: bool,

    // pub min_nr_or_members: usize,
}

impl Config for ClusterShardingConfig {}

impl ClusterShardingConfig {
    pub fn builder() -> ClusterShardingConfigBuilder {
        ClusterShardingConfigBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct ClusterShardingConfigBuilder {
    builder: config::ConfigBuilder<DefaultState>,
}

impl ConfigBuilder for ClusterShardingConfigBuilder {
    type C = ClusterShardingConfig;

    fn add_source<T>(self, source: T) -> anyhow::Result<Self> where T: Source + Send + Sync + 'static {
        Ok(Self { builder: self.builder.add_source(source) })
    }

    fn build(self) -> anyhow::Result<Self::C> {
        let builder = self.builder.add_source(File::from_str(CLUSTER_SHARDING_CONFIG, FileFormat::Toml));
        let cluster_sharding_config = builder.build()?.try_deserialize::<Self::C>()?;
        Ok(cluster_sharding_config)
    }
}