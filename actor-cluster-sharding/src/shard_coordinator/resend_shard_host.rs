use async_trait::async_trait;

use actor_core::actor::context::ActorContext1;
use actor_core::actor_ref::ActorRef;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct ResendShardHost {
    pub(super) shard: ImShardId,
    pub(super) region: ActorRef,
}

#[async_trait]
impl Message for ResendShardHost {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(region) = actor.state.shards.get(&self.shard) {
            if region == &self.region {
                actor.send_host_shard_msg(context, self.shard, self.region);
            }
        }
        Ok(())
    }
}