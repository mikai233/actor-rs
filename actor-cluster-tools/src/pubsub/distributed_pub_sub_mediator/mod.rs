use async_trait::async_trait;
use tracing::trace;

use actor_cluster::cluster::Cluster;
use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};

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
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}