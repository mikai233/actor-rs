use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::collections::hash_map::Entry;
use std::ops::Not;
use tracing::debug;

use crate::shard_coordinator::rebalance_worker::begin_handoff_ack::BeginHandoffAck;
use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[display("BeginHandoff {{ shard: {shard} }}")]
pub(crate) struct BeginHandoff {
    pub(crate) shard: ShardId,
}

impl MessageHandler<ShardRegion> for BeginHandoff {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut ShardRegion::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        debug!(
            "{}: BeginHandOff shard [{}]",
            actor.type_name, message.shard
        );
        if actor.preparing_for_shutdown.not() {
            if let Some(region_ref) = actor.region_by_shard.remove(message.shard.as_str()) {
                if let Entry::Occupied(mut o) = actor.regions.entry(region_ref) {
                    let shards = o.get_mut();
                    shards.remove(message.shard.as_str());
                    if shards.is_empty() {
                        o.remove();
                    }
                }
            }
            let sender = sender.ok_or(anyhow!("Sender is None"))?;
            sender.cast(
                BeginHandoffAck::new(message.shard),
                Some(ctx.myself().clone()),
            );
        } else {
            debug!(
                "{}: Ignoring begin handoff of shard [{}] as preparing to shutdown",
                actor.type_name, message.shard
            );
        }
        Ok(Behavior::same())
    }
}
