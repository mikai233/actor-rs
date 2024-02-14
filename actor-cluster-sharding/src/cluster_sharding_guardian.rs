use async_trait::async_trait;
use tracing::trace;

use actor_cluster::cluster::Cluster;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;

#[derive(Debug)]
pub(crate) struct ClusterShardingGuardian {
    pub(crate) cluster: Cluster,
}

impl ClusterShardingGuardian {
    fn coordinator_singleton_manager_name(enc_name: &str) -> String {
        format!("{}_coordinator", enc_name)
    }

    fn coordinator_path(myself: &ActorRef, enc_name: String) -> String {
        myself.path()
            .child(&Self::coordinator_singleton_manager_name(&enc_name))
            .child("singleton")
            .child("coordinator")
            .to_string_without_address()
    }

    fn start_coordinator_if_needed(
        &self,
        type_name: String,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
        settings: ClusterShardingSettings,
    ) {}
}

#[async_trait]
impl Actor for ClusterShardingGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct Start {
    pub(crate) type_name: String,
    pub(crate) entity_props: Props,
    pub(crate) settings: ClusterShardingSettings,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
    pub(crate) handoff_stop_message: DynMessage,
}

#[async_trait]
impl Message for Start {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct StartProxy {
    pub(crate) type_name: String,
    pub(crate) settings: ClusterShardingSettings,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
}

#[async_trait]
impl Message for StartProxy {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct StartCoordinatorIfNeeded {
    pub(crate) type_name: String,
    pub(crate) settings: ClusterShardingSettings,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
}

#[async_trait]
impl Message for StartCoordinatorIfNeeded {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}