use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::ClusterEvent;
use crate::coordinated_shutdown_leave::CoordinatedShutdownLeave;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = CoordinatedShutdownLeave;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        match self.0 {
            ClusterEvent::MemberLeaving(m) => {
                if actor.cluster.self_unique_address() == &m.addr {
                    actor.done(context);
                }
            }
            ClusterEvent::MemberRemoved(m) => {
                if actor.cluster.self_unique_address() == &m.addr {
                    actor.done(context);
                }
            }
            ClusterEvent::CurrentClusterState { members, .. } => {
                let removed = members.into_values().find(|m| {
                    &m.addr == actor.cluster.self_unique_address() && m.status == MemberStatus::Removed
                }).is_some();
                if removed {
                    actor.done(context);
                }
            }
            _ => {}
        }
        Ok(())
    }
}