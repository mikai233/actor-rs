use async_trait::async_trait;
use bincode::{Decode, Encode};

use kairo_core::Message;
use kairo_core::MessageCodec;
use kairo_core::actor::context::ActorContext;
use kairo_core::actor_ref::ActorRef;

use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ShardHome {
    pub(crate) shard: ShardId,
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for ShardHome {
    type A = ShardRegion;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        actor.receive_shard_home(context, self.shard.into(), self.shard_region)?;
        Ok(())
    }
}
