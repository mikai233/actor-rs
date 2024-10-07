use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_event::MemberEvent;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterHeartbeatReceiver;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} {:?}", context.myself(), self);
        match self.0 {
            MemberEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.unique_address == m.unique_address) {
                    actor.self_member = Some(m.clone());
                }
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
            }
            MemberEvent::CurrentClusterState { self_member, .. } => {
                actor.self_member = Some(self_member);
            }
            MemberEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}