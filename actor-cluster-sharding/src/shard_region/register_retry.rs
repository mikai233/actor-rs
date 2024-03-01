use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, EmptyCodec)]
pub(super) struct RegisterRetry;

#[async_trait]
impl Message for RegisterRetry {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}