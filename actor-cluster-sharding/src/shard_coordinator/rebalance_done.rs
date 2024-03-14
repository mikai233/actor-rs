use async_trait::async_trait;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::state_update::StateUpdate;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct RebalanceDone {
    pub(super) shard: ImShardId,
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
            if let Some(waiting) = actor.waiting_for_shards_to_stop.remove(self.shard.as_str()) {
                for (reply_to, _) in waiting {
                    reply_to.cast_ns(ShardStopped { shard: self.shard.clone().into() });
                }
            }
            if actor.state.shards.contains_key(self.shard.as_str()) {
                actor.update(StateUpdate::ShardHomeDeallocated { shard: self.shard.clone() });
                debug!("{}: Shard [{}] deallocated after", actor.type_name, self.shard);
                actor.clear_rebalance_in_progress(context, self.shard.clone().into());
                //TODO ignore ref
                context.myself().cast(
                    GetShardHome { shard: self.shard.into() },
                    Some(context.system().dead_letters()),
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