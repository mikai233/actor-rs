use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
struct PassivateIntervalTick;

#[async_trait]
impl Message for PassivateIntervalTick {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        todo!()
    }
}