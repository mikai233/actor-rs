use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;

use actor_cluster::cluster::Cluster;
use actor_core::Actor;
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;

pub(crate) mod get_shard_home;
pub(crate) mod shard_stopped;
pub(crate) mod terminate_coordinator;
pub(crate) mod register;
pub(crate) mod register_proxy;

#[derive(Debug)]
pub struct ShardCoordinator {
    type_name: String,
    settings: Arc<ClusterShardingSettings>,
    allocation_strategy: Box<dyn ShardAllocationStrategy>,
    cluster: Cluster,
    rebalance_in_progress: HashMap<String, HashSet<ActorRef>>,
    rebalance_workers: HashSet<ActorRef>,
    un_acked_host_shards: HashMap<String, ()>,
    graceful_shutdown_in_progress: HashSet<ActorRef>,
    alive_regions: HashSet<ActorRef>,
}

impl ShardCoordinator {
    pub(crate) fn new(
        context: &mut ActorContext,
        type_name: String,
        settings: Arc<ClusterShardingSettings>,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
    ) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self {
            type_name,
            settings,
            allocation_strategy,
            cluster,
            rebalance_in_progress: Default::default(),
            rebalance_workers: Default::default(),
            un_acked_host_shards: Default::default(),
            graceful_shutdown_in_progress: Default::default(),
            alive_regions: Default::default(),
        }
    }
}

#[async_trait]
impl Actor for ShardCoordinator {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}