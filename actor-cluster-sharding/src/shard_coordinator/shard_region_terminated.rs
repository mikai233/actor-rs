use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardRegionTerminated(pub(super) Terminated);

impl ShardRegionTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for ShardRegionTerminated {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.region_terminated(context, self.0.actor).await;
        Ok(())
    }
}