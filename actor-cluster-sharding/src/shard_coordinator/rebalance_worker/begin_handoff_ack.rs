use std::any::type_name;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use anyhow::{anyhow, Context as _};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_region::ShardId;

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor
)]
#[display("BeginHandoffAck {{ shard: {shard} }}")]
pub(crate) struct BeginHandoffAck {
    pub(crate) shard: ShardId,
}

impl MessageHandler<RebalanceWorker> for BeginHandoffAck {
    fn handle(actor: &mut RebalanceWorker, ctx: &mut <RebalanceWorker as Actor>::Context, message: Self, sender: Option<ActorRef>, _: &Receive<RebalanceWorker>) -> anyhow::Result<Behavior<RebalanceWorker>> {
        let shard = message.shard;
        let sender = sender.ok_or(anyhow!("Sender is none"))?;
        if actor.shard == shard {
            debug!("{}: BeginHandOffAck for shard [{}] received from [{}].", actor.type_name, actor.shard, sender);
            actor.acked(ctx, &sender);
        } else {
            debug!("{}: Ignore unknown BeginHandOffAck for shard [{}] received from [{}].", actor.type_name, actor.shard, sender);
        }
        Ok(Behavior::same())
    }
}