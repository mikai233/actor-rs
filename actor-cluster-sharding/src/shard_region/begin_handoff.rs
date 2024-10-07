use std::any::type_name;
use std::collections::hash_map::Entry;
use std::ops::Not;

use anyhow::Context as _;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_coordinator::rebalance_worker::begin_handoff_ack::BeginHandoffAck;
use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct BeginHandoff {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for BeginHandoff {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
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
            context.sender()
                .into_result()
                .context(type_name::<BeginHandoff>())
                ?.cast(BeginHandoffAck { shard: self.shard }, Some(context.myself().clone()));
        } else {
            debug!("{}: Ignoring begin handoff of shard [{}] as preparing to shutdown", actor.type_name, self.shard);
        }
        Ok(())
    }
}