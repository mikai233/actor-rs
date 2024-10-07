use async_trait::async_trait;

use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::MemberEvent;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = OnMemberStatusChangedListener;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            MemberEvent::MemberUp(m) if matches!(actor.status, MemberStatus::Up) => {
                if actor.is_triggered(&m) {
                    actor.done(context);
                }
            }
            MemberEvent::MemberRemoved(m) if matches!(actor.status, MemberStatus::Removed) => {
                if actor.is_triggered(&m) {
                    actor.done(context);
                }
            }
            MemberEvent::CurrentClusterState { members, .. } => {
                for (_, m) in members {
                    if actor.is_triggered(&m) {
                        actor.done(context);
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}