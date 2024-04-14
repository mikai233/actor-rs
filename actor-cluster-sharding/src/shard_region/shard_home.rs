use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ShardHome {
    pub(crate) shard: ShardId,
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for ShardHome {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        actor.receive_shard_home(context, self.shard.into(), self.shard_region)?;
        Ok(())
    }
}