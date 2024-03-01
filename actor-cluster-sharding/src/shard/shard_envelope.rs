use async_trait::async_trait;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;
use crate::message_extractor::ShardEntityEnvelope;
use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub(crate) struct ShardEnvelope(pub(crate) ShardEntityEnvelope);

#[async_trait]
impl Message for ShardEnvelope {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}