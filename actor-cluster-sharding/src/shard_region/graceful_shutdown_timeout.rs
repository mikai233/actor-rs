use async_trait::async_trait;
use itertools::Itertools;
use tracing::warn;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct GracefulShutdownTimeout;

#[async_trait]
impl Message for GracefulShutdownTimeout {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shards = actor.shards.keys().join(", ");
        let buffer_size = actor.shard_buffers.total_size();
        warn!(
            "{}: Graceful shutdown of shard region timed out, region will be stopped. Remaining shards [{}], remaining buffered messages [{}]",
            actor.type_name,
            shards,
            buffer_size,
        );
        context.stop(context.myself());
        Ok(())
    }
}