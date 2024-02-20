use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_cluster::cluster::Cluster;
use actor_core::{Actor, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::ActorContext;
use actor_derive::{CMessageCodec, MessageCodec};

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;

#[derive(Debug)]
pub struct ShardCoordinator {
    type_name: String,
    settings: ClusterShardingSettings,
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
        settings: ClusterShardingSettings,
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

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct TerminateCoordinator;

#[async_trait]
impl Message for TerminateCoordinator {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct Register {
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for Register {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct RegisterProxy {
    pub(crate) shard_region_proxy: ActorRef,
}

#[async_trait]
impl Message for RegisterProxy {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}