use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::cluster_core_supervisor::ClusterCoreSupervisor;
use crate::cluster_daemon::get_cluster_core_ref_req::GetClusterCoreRefResp;

#[derive(Debug, Message, derive_more::Display)]
#[display("GetClusterCoreRef")]
pub(crate) struct GetClusterCoreRef;

impl MessageHandler<ClusterCoreSupervisor> for GetClusterCoreRef {
    fn handle(
        actor: &mut ClusterCoreSupervisor,
        ctx: &mut <ClusterCoreSupervisor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterCoreSupervisor>,
    ) -> anyhow::Result<Behavior<ClusterCoreSupervisor>> {
        let core_daemon = match &actor.core_daemon {
            None => actor.create_children(ctx)?,
            Some(core_daemon) => core_daemon.clone(),
        };
        if let Some(sender) = sender {
            sender.cast_ns(GetClusterCoreRefResp(core_daemon));
        }
        Ok(Behavior::same())
    }
}
