use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::MemberEvent;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            MemberEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.unique_address == m.unique_address) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.insert(m.unique_address);
            }
            MemberEvent::MemberPrepareForLeaving(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.unique_address == m.unique_address) {
                    actor.self_member = Some(m.clone());
                }
            }
            MemberEvent::MemberLeaving(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.unique_address == m.unique_address) {
                    actor.self_member = Some(m.clone());
                }
            }
            MemberEvent::MemberRemoved(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.unique_address == m.unique_address) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.remove(&m.unique_address);
            }
            MemberEvent::CurrentClusterState { members, self_member } => {
                actor.self_member = Some(self_member);
                let up_members = members
                    .into_iter()
                    .filter(|(_, m)| m.status == MemberStatus::Up)
                    .map(|(addr, _)| addr);
                actor.active_receivers.extend(up_members);
            }
            MemberEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}