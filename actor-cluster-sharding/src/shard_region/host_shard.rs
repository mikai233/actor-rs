use std::collections::hash_map::Entry;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use ahash::{HashSet, HashSetExt};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::debug;

use actor_core::actor::ctx::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_coordinator::shard_started::ShardStarted;
use crate::shard_region::{ImShardId, ShardRegion};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("HostShard {{ shard: {shard} }}")]
pub(crate) struct HostShard {
    pub(crate) shard: String,
}

impl MessageHandler<ShardRegion> for HostShard {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        let type_name = &actor.type_name;
        let shard: ImShardId = self.shard.into();
        if actor.graceful_shutdown_in_progress {
            debug!(
                "{type_name}: Ignoring Host Shard request for [{shard}] as region is shutting down"
            );
            actor.send_graceful_shutdown_to_coordinator_if_in_progress(ctx)?;
        } else {
            debug!("{type_name}: Host Shard [{shard}]");
            actor
                .region_by_shard
                .insert(shard.clone(), ctx.myself().clone());
            match actor.regions.entry(ctx.myself().clone()) {
                Entry::Occupied(mut o) => {
                    o.get_mut().insert(shard.clone());
                }
                Entry::Vacant(v) => {
                    let mut shards = HashSet::new();
                    shards.insert(shard.clone());
                    v.insert(shards);
                }
            }
            actor.get_shard(ctx, shard.clone())?;
            let sender = sender.ok_or(anyhow!("Sender is None"))?;
            sender.cast_ns(ShardStarted::new(shard.into()));
        }
        Ok(Behavior::same())
    }
}
