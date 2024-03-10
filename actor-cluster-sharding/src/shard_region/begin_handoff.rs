use std::collections::hash_map::Entry;
use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::begin_handoff_ack::BeginHandoffAck;
use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct BeginHandoff {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for BeginHandoff {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("{}: BeginHandOff shard [{}]", actor.type_name, self.shard);
        if actor.preparing_for_shutdown.not() {
            if let Some(region_ref) = actor.region_by_shard.remove(self.shard.as_str()) {
                if let Entry::Occupied(mut o) = actor.regions.entry(region_ref) {
                    let shards = o.get_mut();
                    shards.remove(self.shard.as_str());
                    if shards.is_empty() {
                        o.remove();
                    }
                }
            }
            context.sender().unwrap().cast_ns(BeginHandoffAck { shard: self.shard });
        } else {
            debug!("{}: Ignoring begin handoff of shard [{}] as preparing to shutdown", actor.type_name, self.shard);
        }
        todo!()
    }
}