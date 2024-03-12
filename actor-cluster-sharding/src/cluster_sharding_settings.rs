use std::time::Duration;

use typed_builder::TypedBuilder;

use actor_cluster::cluster::Cluster;
use actor_cluster_tools::singleton::cluster_singleton_manager::ClusterSingletonManagerSettings;
use actor_core::actor::actor_system::ActorSystem;

use crate::config::ClusterShardingConfig;

#[derive(Debug, Clone, TypedBuilder)]
pub struct ClusterShardingSettings {
    pub role: Option<String>,
    pub shard_region_query_timeout: Duration,
    pub coordinator_singleton_settings: ClusterSingletonManagerSettings,
    pub retry_interval: Duration,
    pub handoff_timeout: Duration,
    pub shard_start_timeout: Duration,
}

impl ClusterShardingSettings {
    pub fn create(system: &ActorSystem) -> Self {
        let sharding_config = system.get_config::<ClusterShardingConfig>();
        let settings = Self {
            role: sharding_config.role.clone(),
            shard_region_query_timeout: sharding_config.shard_region_query_timeout,
            coordinator_singleton_settings: sharding_config.coordinator_singleton_settings.clone(),
            retry_interval: sharding_config.retry_interval,
            handoff_timeout: sharding_config.handoff_timeout,
            shard_start_timeout: sharding_config.shard_start_timeout,
        };
        settings
    }

    pub(crate) fn should_host_shard(&self, cluster: &Cluster) -> bool {
        self.role.iter().all(|role| { cluster.self_member().has_role(role) })
    }

    pub(crate) fn should_host_coordinator(&self, cluster: &Cluster) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone)]
struct PassivationStrategySettings {
    idle_entity_settings: IdleSettings,
    active_entity_limit: Option<usize>,

}

#[derive(Debug, Clone)]
struct IdleSettings {
    timeout: Duration,
    interval: Option<Duration>,
}