use async_trait::async_trait;
use tracing::trace;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct DistributedPubSubMediator {}

#[async_trait]
impl Actor for DistributedPubSubMediator {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}