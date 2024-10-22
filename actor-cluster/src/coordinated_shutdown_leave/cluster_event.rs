use actor_core::actor::context::Context;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::cluster_event::MemberEvent;
use crate::coordinated_shutdown_leave::CoordinatedShutdownLeave;
use crate::member::MemberStatus;

#[derive(Debug, Message,derive_more::Display)]
#[display("ClusterEventWrap")]
pub(super) struct ClusterEventWrap(pub(super) dyn MemberEvent);

impl MessageHandler<CoordinatedShutdownLeave> for ClusterEventWrap {
    fn handle(
        actor: &mut CoordinatedShutdownLeave,
        ctx: &mut <CoordinatedShutdownLeave as Actor>::Context,
        message: Self,
        sender: Option<actor_core::actor_ref::ActorRef>,
        _: &actor_core::actor::receive::Receive<CoordinatedShutdownLeave>,
    ) -> anyhow::Result<actor_core::actor::behavior::Behavior<CoordinatedShutdownLeave>> {
        todo!()
    }
}

#[async_trait]
impl Message for ClusterEventWrap {
    type A = CoordinatedShutdownLeave;

    async fn handle(
        self: Box<Self>,
        context: &mut Context,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
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
                let removed = members
                    .into_values()
                    .find(|m| {
                        &m.unique_address == actor.cluster.self_unique_address()
                            && m.status == MemberStatus::Removed
                    })
                    .is_some();
                if removed {
                    actor.done(context);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
