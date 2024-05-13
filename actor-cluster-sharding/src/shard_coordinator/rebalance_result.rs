use ahash::HashSet;
use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct RebalanceResult {
    pub(super) shards: HashSet<ImShardId>,
}

#[async_trait]
impl Message for RebalanceResult {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.continue_rebalance(context, self.shards)?;
        Ok(())
    }
}