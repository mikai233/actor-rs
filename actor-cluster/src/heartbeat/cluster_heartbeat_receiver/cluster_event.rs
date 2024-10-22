use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};

use crate::cluster_event::MemberEvent;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;

#[derive(Debug, Message, derive_more::Display)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

impl MessageHandler<ClusterHeartbeatReceiver> for ClusterEventWrap {
    fn handle(
        actor: &mut ClusterHeartbeatReceiver,
        ctx: &mut <ClusterHeartbeatReceiver as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterHeartbeatReceiver>,
    ) -> anyhow::Result<Behavior<ClusterHeartbeatReceiver>> {
        todo!()
    }
}

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterHeartbeatReceiver;

    async fn handle(
        self: Box<Self>,
        context: &mut Context,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        trace!("{} {:?}", context.myself(), self);
        match self.0 {
            MemberEvent::MemberUp(m) => {
                if actor
                    .self_member
                    .as_ref()
                    .is_some_and(|sm| sm.unique_address == m.unique_address)
                {
                    actor.self_member = Some(m.clone());
                }
            }
            MemberEvent::MemberPrepareForLeaving(m) => {
                if actor
                    .self_member
                    .as_ref()
                    .is_some_and(|sm| sm.unique_address == m.unique_address)
                {
                    actor.self_member = Some(m.clone());
                }
            }
            MemberEvent::MemberLeaving(m) => {
                if actor
                    .self_member
                    .as_ref()
                    .is_some_and(|sm| sm.unique_address == m.unique_address)
                {
                    actor.self_member = Some(m.clone());
                }
            }
            MemberEvent::MemberRemoved(m) => {
                if actor
                    .self_member
                    .as_ref()
                    .is_some_and(|sm| sm.unique_address == m.unique_address)
                {
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
