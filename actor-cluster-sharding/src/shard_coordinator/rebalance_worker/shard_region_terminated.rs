use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::Context;
use actor_core::actor_ref::ActorRef;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;

#[derive(Debug, EmptyCodec)]
pub(crate) struct ShardRegionTerminated {
    pub(crate) region: ActorRef,
}

#[async_trait]
impl Message for ShardRegionTerminated {
    type A = RebalanceWorker;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let region = self.region;
        if !actor.stopping_shard {
            if actor.remaining.contains(&region) {
                debug!(
                    "{}: ShardRegion [{}] terminated while waiting for BeginHandOffAck for shard [{}]",
                    actor.type_name,
                    region,
                    actor.shard,
                );
                actor.acked(context, &region);
            }
        } else if actor.shard_region_from == region {
            debug!(
                "{}: ShardRegion [{}] terminated while waiting for ShardStopped for shard [{}]",
                actor.type_name,
                region,
                actor.shard,
            );
            actor.done(context, true);
        }
        Ok(())
    }
}