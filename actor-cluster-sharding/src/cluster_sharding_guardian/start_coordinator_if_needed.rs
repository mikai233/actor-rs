use std::sync::Arc;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use imstr::ImString;

use actor_core::actor::context::Context;
use actor_core::Message;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;

#[derive(Debug, Message)]
pub(crate) struct StartCoordinatorIfNeeded {
    pub(crate) type_name: ImString,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
}

impl MessageHandler<ClusterShardingGuardian> for StartCoordinatorIfNeeded {
    fn handle(
        actor: &mut ClusterShardingGuardian,
        ctx: &mut <ClusterShardingGuardian as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterShardingGuardian>,
    ) -> anyhow::Result<Behavior<ClusterShardingGuardian>> {
        let Self {
            type_name,
            settings,
            allocation_strategy,
        } = message;
        actor.start_coordinator_if_needed(context, type_name, allocation_strategy, settings)?;
        Ok(Behavior::same())
    }
}
