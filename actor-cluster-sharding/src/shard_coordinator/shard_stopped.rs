use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ShardId;

#[derive(Debug, Decode, Encode, MessageCodec)]
pub(crate) struct ShardStopped {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for ShardStopped {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}