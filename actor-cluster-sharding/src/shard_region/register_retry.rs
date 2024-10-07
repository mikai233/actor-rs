use async_trait::async_trait;

use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, EmptyCodec)]
pub(super) struct RegisterRetry;

#[async_trait]
impl Message for RegisterRetry {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.coordinator.is_none() {
            actor.register(context)?;
            actor.scheduler_next_registration(context);
        }
        Ok(())
    }
}