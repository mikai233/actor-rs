use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::debug;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("ShardRegionTerminated {{ region: {region} }}")]
pub(crate) struct ShardRegionTerminated {
    pub(crate) region: ActorRef,
}

impl MessageHandler<RebalanceWorker> for ShardRegionTerminated {
    fn handle(actor: &mut RebalanceWorker, ctx: &mut <RebalanceWorker as Actor>::Context, message: Self, sender: Option<ActorRef>, _: &Receive<RebalanceWorker>) -> anyhow::Result<Behavior<RebalanceWorker>> {
        let region = message.region;
        if !actor.stopping_shard {
            if actor.remaining.contains(&region) {
                debug!(
                    "{}: ShardRegion [{}] terminated while waiting for BeginHandOffAck for shard [{}]",
                    actor.type_name,
                    region,
                    actor.shard,
                );
                actor.acked(ctx, &region);
            }
        } else if actor.shard_region_from == region {
            debug!(
                "{}: ShardRegion [{}] terminated while waiting for ShardStopped for shard [{}]",
                actor.type_name,
                region,
                actor.shard,
            );
            actor.done(ctx, true);
        }
        Ok(Behavior::same())
    }
}