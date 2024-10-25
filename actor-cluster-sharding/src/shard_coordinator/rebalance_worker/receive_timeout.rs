use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::debug;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("ReceiveTimeout")]
pub(super) struct ReceiveTimeout;

impl MessageHandler<RebalanceWorker> for ReceiveTimeout {
    fn handle(actor: &mut RebalanceWorker, ctx: &mut <RebalanceWorker as Actor>::Context, message: Self, sender: Option<ActorRef>, _: &Receive<RebalanceWorker>) -> anyhow::Result<Behavior<RebalanceWorker>> {
        if actor.is_rebalance {
            debug!("{}: Rebalance of [{}] from [{}] timed out", actor.type_name, actor.shard, actor.shard_region_from);
        } else {
            debug!("{}: Shutting down [{}] shard from [{}] timed out", actor.type_name, actor.shard, actor.shard_region_from);
        }
        actor.done(ctx, false);
        Ok(Behavior::same())
    }
}