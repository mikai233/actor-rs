use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::{ImEntityId, ShardRegion};
use actor_core::actor::behavior::Behavior;
use actor_core::actor::props::PropsBuilder;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::message::DynMessage;
use actor_core::Message;
use anyhow::anyhow;
use imstr::ImString;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug, Message)]
pub(crate) struct Start {
    pub(crate) type_name: ImString,
    pub(crate) entity_props: PropsBuilder<ImEntityId>,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
    pub(crate) handoff_stop_message: DynMessage,
}

impl Display for Start {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Start {{ type_name: {}, entity_props: {}, settings: {}, message_extractor: {}, allocation_strategy: {}, handoff_stop_message: {} }}",
            self.type_name,
            self.entity_props.name,
            self.settings,
            self.message_extractor,
            self.allocation_strategy,
            self.handoff_stop_message,
        )
    }
}

impl MessageHandler<ClusterShardingGuardian> for Start {
    fn handle(
        actor: &mut ClusterShardingGuardian,
        ctx: &mut <ClusterShardingGuardian as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterShardingGuardian>,
    ) -> anyhow::Result<Behavior<ClusterShardingGuardian>> {
        let Self {
            type_name,
            entity_props,
            settings,
            message_extractor,
            allocation_strategy,
            handoff_stop_message,
        } = message;
        let shard_region = match ctx.child(&type_name) {
            None => {
                actor.start_coordinator_if_needed(ctx, type_name.clone(), allocation_strategy, settings.clone())?;
                let coordinator_path = ClusterShardingGuardian::coordinator_path(ctx.myself(), &type_name);
                ctx.spawn(
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
        let started = Started::new(shard_region);
        let sender = sender.ok_or(anyhow!("Sender is None"))?;
        sender.cast_ns(started);
        Ok(Behavior::same())
    }
}