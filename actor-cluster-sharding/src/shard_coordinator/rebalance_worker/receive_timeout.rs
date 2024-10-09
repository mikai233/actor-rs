use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::Context;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;

#[derive(Debug, Clone, EmptyCodec)]
pub(super) struct ReceiveTimeout;

#[async_trait]
impl Message for ReceiveTimeout {
    type A = RebalanceWorker;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.is_rebalance {
            debug!("{}: Rebalance of [{}] from [{}] timed out", actor.type_name, actor.shard, actor.shard_region_from);
        } else {
            debug!("{}: Shutting down [{}] shard from [{}] timed out", actor.type_name, actor.shard, actor.shard_region_from);
        }
        actor.done(context, false);
        Ok(())
    }
}