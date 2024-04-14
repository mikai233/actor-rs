use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::ClusterEvent;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterHeartbeatReceiver;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        trace!("{} {:?}", context.myself(), self);
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
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
            }
            ClusterEvent::CurrentClusterState { self_member, .. } => {
                actor.self_member = Some(self_member);
            }
            ClusterEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}