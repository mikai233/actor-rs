use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct RegionStopped {
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for RegionStopped {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard_region = self.shard_region;
        debug!("{}: ShardRegion stopped: [{}]", actor.type_name, shard_region);
        actor.region_terminated(context, shard_region).await;
        Ok(())
    }
}