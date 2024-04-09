use std::sync::Arc;

use async_trait::async_trait;
use imstr::ImString;
use tracing::trace;

use actor_cluster::cluster::Cluster;
use actor_cluster_tools::singleton::cluster_singleton_manager::ClusterSingletonManager;
use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::PropsBuilder;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::terminate_coordinator::TerminateCoordinator;

pub(crate) mod start_coordinator_if_needed;
pub(crate) mod start_proxy;
pub(crate) mod start;
pub(crate) mod started;

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
            //TODO
            // .child("coordinator")
            .to_string_without_address()
    }

    fn start_coordinator_if_needed(
        &self,
        context: &mut ActorContext,
        type_name: ImString,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
        settings: Arc<ClusterShardingSettings>,
    ) -> eyre::Result<()> {
        let mgr_name = Self::coordinator_singleton_manager_name(&type_name);
        if settings.should_host_coordinator(&Cluster::get(context.system())) && context.child(&mgr_name).is_none() {
            let mut singleton_settings = settings.coordinator_singleton_settings.clone();
            singleton_settings.singleton_name = "singleton".to_string();
            singleton_settings.role = settings.role.clone();
            let coordinator_props = PropsBuilder::new_wit_ctx(move |ctx, ()| {
                let type_name = type_name.clone();
                let settings = settings.clone();
                let allocation_strategy = allocation_strategy.clone();
                ShardCoordinator::new(ctx, type_name, settings, allocation_strategy)
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
    async fn started(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}