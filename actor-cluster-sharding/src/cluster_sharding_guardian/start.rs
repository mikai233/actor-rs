use std::sync::Arc;

use async_trait::async_trait;
use imstr::ImString;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor::props::PropsBuilder;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::{ImEntityId, ShardRegion};

#[derive(Debug, EmptyCodec)]
pub(crate) struct Start {
    pub(crate) type_name: ImString,
    pub(crate) entity_props: PropsBuilder<ImEntityId>,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
    pub(crate) handoff_stop_message: DynMessage,
}

#[async_trait]
impl Message for Start {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
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
                        entity_props.into(),
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
        context.sender().into_result()?.cast_orphan_ns(started);
        Ok(())
    }
}