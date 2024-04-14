use async_trait::async_trait;
use tracing::debug;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardTerminated(pub(super) Terminated);

impl ShardTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for ShardTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let shard = self.0;
        if actor.shards_by_ref.contains_key(&shard) {
            if let Some(shard_id) = actor.shards_by_ref.remove(&shard) {
                actor.shards.remove(&shard_id);
                actor.starting_shards.remove(&shard_id);
                //TODO passivation strategy
                let type_name = &actor.type_name;
                match actor.handing_off.remove(&shard) {
                    true => {
                        debug!("{type_name}: Shard [{shard_id}] handoff complete")
                    }
                    false => {
                        debug!("{type_name}: Shard [{shard_id}] terminated while not being handed off");
                    }
                }
            }
        }
        Ok(())
    }
}