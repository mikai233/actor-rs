use async_trait::async_trait;
use tracing::trace;

use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::{Actor, DynMessage};

pub mod count;
pub mod get_topics;
mod per_grouping_buffer;
pub mod publish;
pub mod send;
pub mod send_to_all;
pub mod subscribe;
mod topic;
pub mod unsubscribe;

#[derive(Debug)]
pub struct DistributedPubSubMediator {
    // cluster: Cluster,
    // removed_time_to_live_millis: u64,
}

#[async_trait]
impl Actor for DistributedPubSubMediator {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(
        &mut self,
        context: &mut ActorContext,
        message: DynMessage,
    ) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
