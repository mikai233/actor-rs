use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("ShardStarted {{ shard: {shard} }}")]
pub(crate) struct ShardStarted {
    pub(crate) shard: ShardId,
}

impl MessageHandler<ShardCoordinator> for ShardStarted {
    fn handle(
        actor: &mut ShardCoordinator,
        _: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        if let Some(key) = actor.un_acked_host_shards.remove(message.shard.as_str()) {
            key.cancel();
        }
        Ok(Behavior::same())
    }
}
