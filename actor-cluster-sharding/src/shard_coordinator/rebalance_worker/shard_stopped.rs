use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::Context;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_region::ShardId;

#[derive(Debug, Decode, Encode, MessageCodec)]
pub(crate) struct ShardStopped {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for ShardStopped {
    type A = RebalanceWorker;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard = self.shard;
        if shard == actor.shard.as_str() {
            if actor.stopping_shard {
                debug!("{}: ShardStopped {}", actor.type_name, shard);
                actor.done(context, true);
            } else {
                debug!("{}: Ignore ShardStopped {} because RebalanceWorker not in stopping shard", actor.type_name, shard);
            }
        } else {
            debug!("{}: Ignore unknown ShardStopped {} for shard {}", actor.type_name, shard, actor.shard);
        }
        Ok(())
    }
}