use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::heartbeat::cluster_heartbeat_receiver::heartbeat::Heartbeat;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::MemberStatus;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("HeartbeatTick")]
pub(super) struct HeartbeatTick;

impl MessageHandler<ClusterHeartbeatSender> for HeartbeatTick {
    fn handle(
        actor: &mut ClusterHeartbeatSender,
        ctx: &mut <ClusterHeartbeatSender as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterHeartbeatSender>,
    ) -> anyhow::Result<Behavior<ClusterHeartbeatSender>> {
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                for receiver in &actor.active_receivers {
                    let path = ActorSelectionPath::FullPath(ClusterHeartbeatReceiver::path(
                        receiver.address.clone(),
                    ));
                    let sel = ctx.actor_selection(path)?;
                    let heartbeat = Heartbeat::new(self_member.unique_address.clone());
                    sel.tell(Box::new(heartbeat), Some(ctx.myself().clone()));
                }
            }
        }
        Ok(Behavior::same())
    }
}
