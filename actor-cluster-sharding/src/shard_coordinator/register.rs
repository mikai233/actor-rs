use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::register_ack::RegisterAck;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct Register {
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for Register {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let region = self.shard_region;
        if actor.is_member(&region) {
            debug!("{}: ShardRegion registered: [{}]", actor.type_name, region);
            actor.alive_regions.insert(region.clone());
            if actor.state.regions.contains_key(&region) {
                region.cast_ns(RegisterAck { coordinator: context.myself().clone() });
                
            } else {
                actor.graceful_shutdown_in_progress.remove(&region);
            }
        } else {
            debug!(
                "{}: ShardRegion [{}] was not registered since the coordinator currently does not know about a node of that region",
                actor.type_name,
                region,
            );
        }
        Ok(())
    }
}