use async_trait::async_trait;

use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::MemberEvent;
use crate::coordinated_shutdown_leave::CoordinatedShutdownLeave;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) dyn MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = CoordinatedShutdownLeave;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            MemberEvent::MemberLeaving(m) => {
                if actor.cluster.self_unique_address() == &m.unique_address {
                    actor.done(context);
                }
            }
            MemberEvent::MemberRemoved(m) => {
                if actor.cluster.self_unique_address() == &m.unique_address {
                    actor.done(context);
                }
            }
            MemberEvent::CurrentClusterState { members, .. } => {
                let removed = members.into_values().find(|m| {
                    &m.unique_address == actor.cluster.self_unique_address() && m.status == MemberStatus::Removed
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