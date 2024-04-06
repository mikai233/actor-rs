use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ShardId;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ShardStarted {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for ShardStarted {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if let Some(key) = actor.un_acked_host_shards.remove(self.shard.as_str()) {
            key.cancel();
        }
        Ok(())
    }
}