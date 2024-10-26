use std::any::{type_name, Any};

use crate::message_extractor::{ShardEnvelopePacket, ShardEnvelope};
use crate::shard::Shard;
use crate::shard_region::ShardRegion;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use tracing::debug;

impl MessageHandler<ShardRegion> for ShardEnvelope {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if actor.graceful_shutdown_in_progress {
            debug!(
                "{}: Ignore {} when ShardRegion is graceful shutdown in progress",
                actor.type_name, message
            );
        } else {
            actor.deliver_message(ctx, message)?;
        }
        Ok(Behavior::same())
    }
}
