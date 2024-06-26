use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::ClusterEvent;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.insert(m.addr);
            }
            ClusterEvent::MemberPrepareForLeaving(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
            }
            ClusterEvent::MemberLeaving(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
            }
            ClusterEvent::MemberRemoved(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.remove(&m.addr);
            }
            ClusterEvent::CurrentClusterState { members, self_member } => {
                actor.self_member = Some(self_member);
                let up_members = members
                    .into_iter()
                    .filter(|(_, m)| m.status == MemberStatus::Up)
                    .map(|(addr, _)| addr);
                actor.active_receivers.extend(up_members);
            }
            ClusterEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}