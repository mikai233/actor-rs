use std::collections::hash_map::Entry;

use ahash::{HashSet, HashSetExt};
use async_trait::async_trait;
use bincode::{Decode, Encode};
use eyre::Context as _;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::shard_started::ShardStarted;
use crate::shard_region::{ImShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct HostShard {
    pub(crate) shard: String,
}

#[async_trait]
impl Message for HostShard {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let type_name = &actor.type_name;
        let shard: ImShardId = self.shard.into();
        if actor.graceful_shutdown_in_progress {
            debug!("{type_name}: Ignoring Host Shard request for [{shard}] as region is shutting down");
            actor.send_graceful_shutdown_to_coordinator_if_in_progress(context)?;
        } else {
            debug!("{type_name}: Host Shard [{shard}]");
            actor.region_by_shard.insert(shard.clone(), context.myself().clone());
            match actor.regions.entry(context.myself().clone()) {
                Entry::Occupied(mut o) => {
                    o.get_mut().insert(shard.clone());
                }
                Entry::Vacant(v) => {
                    let mut shards = HashSet::new();
                    shards.insert(shard.clone());
                    v.insert(shards);
                }
            }
            actor.get_shard(context, shard.clone())?;
            context.sender()
                .into_result()
                .context(std::any::type_name::<HostShard>())
                ?.cast_ns(ShardStarted { shard: shard.into() });
        }
        Ok(())
    }
}