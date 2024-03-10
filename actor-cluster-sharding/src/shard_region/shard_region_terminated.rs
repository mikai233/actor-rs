use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardRegionTerminated(pub(super) ActorRef);

impl Terminated for ShardRegionTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for ShardRegionTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard_region = self.0;
        if actor.regions.contains_key(&shard_region) {
            if let Some(shards) = actor.regions.remove(&shard_region) {
                for shard in &shards {
                    actor.region_by_shard.remove(shard);
                }
                let type_name = &actor.type_name;
                let size = shards.len();
                let shard_str = shards.iter()
                    .map(|shard| shard.as_str())
                    .collect::<Vec<_>>().join(", ");
                debug!("{type_name}: Region [{shard_region}] terminated with [{size}] shards [{shard_str}]");
            }
        }
        Ok(())
    }
}