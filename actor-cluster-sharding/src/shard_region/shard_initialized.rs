use std::any::type_name;

use crate::shard_region::deliver_target::DeliverTarget;
use crate::shard_region::{ImShardId, ShardRegion};
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use anyhow::anyhow;
use tracing::debug;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("ShardInitialized {{ shard_id: {shard_id} }}")]
pub(crate) struct ShardInitialized {
    pub(crate) shard_id: ImShardId,
}

impl MessageHandler<ShardRegion> for ShardInitialized {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut ShardRegion::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        let shard = sender.ok_or(anyhow!("Sender is None"))?;
        debug!(
            "{}: Shard was initialized [{}]",
            actor.type_name, message.shard_id
        );
        actor.starting_shards.remove(&message.shard_id);
        actor.deliver_buffered_messages(&message.shard_id, DeliverTarget::Shard(&shard));
        Ok(Behavior::same())
    }
}
