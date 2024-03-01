use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;

use crate::message_extractor::ShardEntityEnvelope;
use crate::shard_region::ShardRegion;

#[async_trait]
impl Message for ShardEntityEnvelope {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.deliver_message(context, *self)?;
        Ok(())
    }
}