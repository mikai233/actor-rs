use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardRegionProxyTerminated(pub(super) Terminated);

impl ShardRegionProxyTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for ShardRegionProxyTerminated {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        actor.region_proxy_terminated(context, self.0.actor).await;
        Ok(())
    }
}