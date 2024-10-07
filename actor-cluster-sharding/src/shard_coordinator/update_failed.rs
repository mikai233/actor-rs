use async_trait::async_trait;

use actor_core::{EmptyCodec, Message};
use actor_core::actor::context::ActorContext1;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct UpdateFailed;

#[async_trait]
impl Message for UpdateFailed {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.update(context, None).await;
        Ok(())
    }
}