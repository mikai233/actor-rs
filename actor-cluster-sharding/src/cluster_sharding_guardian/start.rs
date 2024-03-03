use std::sync::Arc;

use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::PropsBuilderSync;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::{EntityId, ShardRegion};

#[derive(Debug, EmptyCodec)]
pub(crate) struct Start {
    pub(crate) type_name: String,
    pub(crate) entity_props: PropsBuilderSync<EntityId>,
    pub(crate) settings: Arc<ClusterShardingSettings>,
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
        context.sender().as_result()?.resp(started);
        Ok(())
    }
}