use async_trait::async_trait;
use tracing::info;

use actor_cluster::cluster_event::MemberEvent;
use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = Shard;

    async fn handle(self: Box<Self>, _context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        if matches!(self.0, MemberEvent::MemberPrepareForLeaving(_)) {
            if !actor.preparing_for_shutdown {
                info!("{}: Preparing for shutdown", actor.type_name);
                actor.preparing_for_shutdown = true;
            }
        }
        Ok(())
    }
}