use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("RegisterRetry")]
pub(super) struct RegisterRetry;

impl MessageHandler<ShardRegion> for RegisterRetry {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if actor.coordinator.is_none() {
            actor.register(ctx)?;
            actor.scheduler_next_registration(ctx);
        }
        Ok(Behavior::same())
    }
}