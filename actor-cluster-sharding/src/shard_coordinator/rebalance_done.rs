use async_trait::async_trait;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::shard_stopped::ShardStopped;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct RebalanceDone {
    pub(super) shard: ShardId,
    pub(super) ok: bool,
}

#[async_trait]
impl Message for RebalanceDone {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let sender = context.sender().unwrap();
        actor.rebalance_workers.remove(sender);
        if self.ok {
            debug!("{}: Shard [{}] deallocation completed successfully.", actor.type_name, self.shard);
            if let Some(waiting) = actor.waiting_for_shards_to_stop.remove(&self.shard) {
                for reply_to in waiting {
                    reply_to.cast_ns(ShardStopped { shard: self.shard.clone() });
                }
            }
            if actor.state.shards.contains_key(&self.shard) {
                //TODO update state
                actor.clear_rebalance_in_progress(context, self.shard);
            } else {
                actor.clear_rebalance_in_progress(context, self.shard);
            }
        } else {
            warn!(
                "{}: Shard [{}] deallocation didn't complete within [{:?}].",
                actor.type_name,
                self.shard,
                actor.settings.handoff_timeout,
            );
            actor.state.shards.get(&self.shard).foreach(|region| {
                actor.graceful_shutdown_in_progress.remove(region);
            });
            actor.clear_rebalance_in_progress(context, self.shard);
        }
        Ok(())
    }
}