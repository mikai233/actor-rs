use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

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

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.region_proxy_terminated(self.0.actor).await;
        Ok(())
    }
}