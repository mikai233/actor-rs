use async_trait::async_trait;
use bincode::{Decode, Encode};

use kairo_core::CMessageCodec;
use kairo_core::Message;
use kairo_core::actor::context::ActorContext;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct TerminateCoordinator;

#[async_trait]
impl Message for TerminateCoordinator {
    type A = ShardCoordinator;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        actor.terminate(context);
        Ok(())
    }
}
