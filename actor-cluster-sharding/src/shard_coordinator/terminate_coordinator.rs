use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext1;
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct TerminateCoordinator;

#[async_trait]
impl Message for TerminateCoordinator {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.terminate(context);
        Ok(())
    }
}