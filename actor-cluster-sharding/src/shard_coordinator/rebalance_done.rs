use std::any::type_name;

use async_trait::async_trait;
use eyre::Context as _;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct RebalanceDone {
    pub(super) shard: ImShardId,
    pub(super) ok: bool,
}

#[async_trait]
impl Message for RebalanceDone {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let sender = context.sender().into_result().context(type_name::<RebalanceDone>())?;
        actor.rebalance_workers.remove(sender);
        if self.ok {
            debug!("{}: Shard [{}] deallocation completed successfully.", actor.type_name, self.shard);
            if let Some(waiting) = actor.waiting_for_shards_to_stop.remove(self.shard.as_str()) {
                for (reply_to, _) in waiting {
                    reply_to.cast_ns(ShardStopped { shard: self.shard.clone().into() });
                }
            }
            if actor.state.shards.contains_key(self.shard.as_str()) {
                actor.update_state(context, ShardState::ShardHomeDeallocated { shard: self.shard.clone() }).await;
                debug!("{}: Shard [{}] deallocated after", actor.type_name, self.shard);
                actor.clear_rebalance_in_progress(context, self.shard.clone().into());
                context.myself().cast(
                    GetShardHome { shard: self.shard.into() },
                    Some(actor.ignore_ref.clone()),
                );
            } else {
                actor.clear_rebalance_in_progress(context, self.shard.into());
            }
        } else {
            warn!(
                "{}: Shard [{}] deallocation didn't complete within [{:?}].",
                actor.type_name,
                self.shard,
                actor.settings.handoff_timeout,
            );
            actor.state.shards.get(self.shard.as_str()).foreach(|region| {
                actor.graceful_shutdown_in_progress.remove(region);
            });
            actor.clear_rebalance_in_progress(context, self.shard.into());
        }
        Ok(())
    }
}