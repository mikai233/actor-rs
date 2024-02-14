use async_trait::async_trait;
use tracing::trace;

use actor_cluster::cluster::Cluster;
use actor_cluster_tools::singleton::cluster_singleton_manager::ClusterSingletonManager;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_sharding::ClusterSharding;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_coordinator::{ShardCoordinator, TerminateCoordinator};
use crate::shard_region::ShardRegion;

#[derive(Debug)]
pub(crate) struct ClusterShardingGuardian {
    pub(crate) cluster: Cluster,
}

impl ClusterShardingGuardian {
    fn coordinator_singleton_manager_name(enc_name: &str) -> String {
        format!("{}_coordinator", enc_name)
    }

    fn coordinator_path(myself: &ActorRef, enc_name: &str) -> String {
        myself.path()
            .child(&Self::coordinator_singleton_manager_name(enc_name))
            .child("singleton")
            .child("coordinator")
            .to_string_without_address()
    }

    fn start_coordinator_if_needed(
        &self,
        context: &mut ActorContext,
        type_name: String,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
        settings: ClusterShardingSettings,
    ) -> anyhow::Result<()> {
        let mgr_name = Self::coordinator_singleton_manager_name(&type_name);
        if settings.should_host_coordinator(&Cluster::get(context.system())) && context.child(&mgr_name).is_none() {
            let mut singleton_settings = settings.coordinator_singleton_settings.clone();
            singleton_settings.singleton_name = "singleton".to_string();
            singleton_settings.role = settings.role.clone();
            let coordinator_props = Props::create(move |ctx| {
                let coordinator = ShardCoordinator::new(
                    ctx,
                    type_name.clone(),
                    settings.clone(),
                    allocation_strategy.clone(),
                );
                Ok(coordinator)
            });
            context.spawn(
                ClusterSingletonManager::props(
                    coordinator_props,
                    DynMessage::user(TerminateCoordinator),
                    singleton_settings,
                )?,
                mgr_name,
            )?;
        }
        Ok(())
    }
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
        let Self {
            type_name,
            entity_props,
            settings,
            message_extractor,
            allocation_strategy,
            handoff_stop_message,
        } = *self;
        let shard_region = match context.child(&type_name) {
            None => {
                actor.start_coordinator_if_needed(context, type_name.clone(), allocation_strategy, settings.clone())?;
                let coordinator_path = ClusterShardingGuardian::coordinator_path(context.myself(), &type_name);
                context.spawn(
                    ShardRegion::props(
                        type_name.clone(),
                        entity_props,
                        settings,
                        coordinator_path,
                        message_extractor,
                        handoff_stop_message,
                    ),
                    type_name,
                )?
            }
            Some(shard_region) => { shard_region }
        };
        let started = Started { shard_region };
        context.sender().as_result()?.resp(started);
        Ok(())
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
        let Self { type_name, settings, message_extractor } = *self;
        let enc_name = ClusterSharding::proxy_name(&type_name);
        let coordinator_path = ClusterShardingGuardian::coordinator_path(context.myself(), &enc_name);
        let shard_region = match context.child(&enc_name) {
            None => {
                context.spawn(
                    ShardRegion::proxy_props(
                        enc_name.clone(),
                        settings,
                        coordinator_path,
                        message_extractor,
                    ),
                    enc_name,
                )?
            }
            Some(shard_region) => { shard_region }
        };
        let started = Started { shard_region };
        context.sender().as_result()?.resp(started);
        Ok(())
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
        let Self { type_name, settings, allocation_strategy } = *self;
        actor.start_coordinator_if_needed(context, type_name, allocation_strategy, settings)?;
        Ok(())
    }
}