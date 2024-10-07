use anyhow::Context as _;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{debug, warn};

use actor_core::{CodecMessage, Message};
use actor_core::actor::context::{ActorContext1, ActorContext, ContextExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::MessageCodec;

use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_region::ShardRegion;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct Handoff {
    pub(crate) shard: String,
}

#[async_trait]
impl Message for Handoff {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let type_name = &actor.type_name;
        let shard_id = self.shard.into();
        debug!("{type_name}: Handoff shard [{shard_id}]");
        if actor.shard_buffers.contains_key(&shard_id) {
            let dropped = actor.shard_buffers.drop_to_dead_letters(
                &shard_id,
                "Avoiding reordering of buffered messages at shard handoff".to_string(),
                context.system().dead_letters(),
            );
            if dropped > 0 {
                let type_name = &actor.type_name;
                warn!("{type_name}: Dropping [{dropped}] buffered messages to shard [{shard_id}] during hand off to avoid re-ordering")
            }
        }
        match actor.shards.get(&shard_id) {
            None => {
                context.sender()
                    .into_result()
                    .context(std::any::type_name::<Handoff>())
                    ?.cast_ns(ShardStopped { shard: shard_id.into() });
            }
            Some(shard) => {
                actor.handing_off.insert(shard.clone());
                context.forward(shard, crate::shard::handoff::Handoff { shard: shard_id.into() }.into_dyn());
            }
        }
        Ok(())
    }
}