use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_region::ShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor
)]
pub(crate) struct ShardStopped {
    pub(crate) shard: ShardId,
}

impl MessageHandler<RebalanceWorker> for ShardStopped {
    fn handle(actor: &mut RebalanceWorker, ctx: &mut <RebalanceWorker as Actor>::Context, message: Self, sender: Option<ActorRef>, _: &Receive<RebalanceWorker>) -> anyhow::Result<Behavior<RebalanceWorker>> {
        let shard = message.shard;
        if shard == actor.shard.as_str() {
            if actor.stopping_shard {
                debug!("{}: ShardStopped {}", actor.type_name, shard);
                actor.done(ctx, true);
            } else {
                debug!("{}: Ignore ShardStopped {} because RebalanceWorker not in stopping shard", actor.type_name, shard);
            }
        } else {
            debug!("{}: Ignore unknown ShardStopped {} for shard {}", actor.type_name, shard, actor.shard);
        }
        Ok(Behavior::same())
    }
}