use async_trait::async_trait;

use kairo_core::EmptyCodec;
use kairo_core::Message;
use kairo_core::actor::context::ActorContext;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
struct PassivateIntervalTick;

#[async_trait]
impl Message for PassivateIntervalTick {
    type A = Shard;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
