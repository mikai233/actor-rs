use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::ActorContext1;
use actor_core::actor_ref::ActorRef;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct AllocateShardResult {
    pub(super) shard: ImShardId,
    pub(super) shard_region: Option<ActorRef>,
    pub(super) get_shard_home_sender: ActorRef,
}

#[async_trait]
impl Message for AllocateShardResult {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { shard, shard_region, get_shard_home_sender } = *self;
        match shard_region {
            None => {
                debug!("{}: Shard [{}] allocation failed. It will be retried.", actor.type_name, shard)
            }
            Some(shard_region) => {
                actor.continue_get_shard_home(context, shard, shard_region, get_shard_home_sender).await;
            }
        }
        Ok(())
    }
}