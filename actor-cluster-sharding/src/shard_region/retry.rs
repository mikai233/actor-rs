use std::ops::Not;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;

use actor_core::actor_ref::ActorRef;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("Retry")]
pub(super) struct Retry;

impl MessageHandler<ShardRegion> for Retry {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if actor.shard_buffers.is_empty().not() {
            actor.retry_count += 1;
        }
        if actor.coordinator.is_none() {
            actor.register(ctx)?;
        } else {
            actor.try_request_shard_buffer_homes(ctx);
        }
        actor.send_graceful_shutdown_to_coordinator_if_in_progress(ctx)?;
        actor.try_complete_graceful_shutdown_if_in_progress(ctx);
        Ok(Behavior::same())
    }
}
