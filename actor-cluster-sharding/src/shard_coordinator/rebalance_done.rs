use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use anyhow::anyhow;
use std::any::type_name;
use tracing::{debug, warn};

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("RebalanceDone {{ shard: {shard} ok: {ok} }}")]
pub(super) struct RebalanceDone {
    pub(super) shard: ImShardId,
    pub(super) ok: bool,
}

impl MessageHandler<ShardCoordinator> for RebalanceDone {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let sender = sender.ok_or(anyhow!("Sender is none"))?;
        actor.rebalance_workers.remove(&sender);
        if message.ok {
            debug!(
                "{}: Shard [{}] deallocation completed successfully.",
                actor.type_name, message.shard
            );
            if let Some(waiting) = actor
                .waiting_for_shards_to_stop
                .remove(message.shard.as_str())
            {
                for (reply_to, _) in waiting {
                    reply_to.cast_ns(ShardStopped {
                        shard: message.shard.clone().into(),
                    });
                }
            }
            if actor.state.shards.contains_key(message.shard.as_str()) {
                actor
                    .update_state(
                        ctx,
                        ShardState::ShardHomeDeallocated {
                            shard: message.shard.clone(),
                        },
                    )
                    .await;
                debug!(
                    "{}: Shard [{}] deallocated after",
                    actor.type_name, message.shard
                );
                actor.clear_rebalance_in_progress(ctx, self.shard.clone().into());
                ctx.myself().cast(
                    GetShardHome {
                        shard: message.shard.into(),
                    },
                    Some(actor.ignore_ref.clone()),
                );
            } else {
                actor.clear_rebalance_in_progress(ctx, message.shard.into());
            }
        } else {
            warn!(
                "{}: Shard [{}] deallocation didn't complete within [{:?}].",
                actor.type_name, message.shard, actor.settings.handoff_timeout,
            );
            if let Some(region) = actor.state.shards.get(message.shard.as_str()) {
                actor.graceful_shutdown_in_progress.remove(region);
            }
            actor.clear_rebalance_in_progress(ctx, message.shard.into());
        }
        Ok(Behavior::same())
    }
}
