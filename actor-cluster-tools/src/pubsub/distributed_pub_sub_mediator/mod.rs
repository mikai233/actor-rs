use async_trait::async_trait;
use tracing::trace;

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext1, ActorContext};

pub mod subscribe;
pub mod unsubscribe;
pub mod publish;
pub mod send;
pub mod send_to_all;
pub mod get_topics;
pub mod count;
mod topic;
mod per_grouping_buffer;

#[derive(Debug)]
pub struct DistributedPubSubMediator {
    // cluster: Cluster,
    // removed_time_to_live_millis: u64,
}

#[async_trait]
impl Actor for DistributedPubSubMediator {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}