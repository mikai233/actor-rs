use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardRegionProxyTerminated(pub(super) ActorRef);

impl Terminated for ShardRegionProxyTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for ShardRegionProxyTerminated {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.region_proxy_terminated(self.0);
        Ok(())
    }
}