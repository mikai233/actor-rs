use async_trait::async_trait;
use itertools::Itertools;
use tracing::debug;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardRegionTerminated(pub(super) Terminated);

impl ShardRegionTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for ShardRegionTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard_region = self.0.actor;
        if actor.regions.contains_key(&shard_region) {
            if let Some(shards) = actor.regions.remove(&shard_region) {
                for shard in &shards {
                    actor.region_by_shard.remove(shard);
                }
                let type_name = &actor.type_name;
                let size = shards.len();
                let shard_str = shards.iter()
                    .join(", ");
                debug!("{type_name}: Region [{shard_region}] terminated with [{size}] shards [{shard_str}]");
            }
        }
        Ok(())
    }
}