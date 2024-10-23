use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use anyhow::anyhow;

use crate::cluster_core_supervisor::get_cluster_core_ref::GetClusterCoreRef;
use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, Message, derive_more::Display)]
#[display("GetClusterCoreRefReq")]
pub(crate) struct GetClusterCoreRefReq;

impl MessageHandler<ClusterDaemon> for GetClusterCoreRefReq {
    fn handle(
        actor: &mut ClusterDaemon,
        ctx: &mut <ClusterDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterDaemon>,
    ) -> anyhow::Result<Behavior<ClusterDaemon>> {
        if actor.core_supervisor.is_none() {
            actor.create_children(ctx)?;
        }
        let core_supervisor = actor
            .core_supervisor
            .as_ref()
            .ok_or(anyhow!("core_supervisor is None"))?;
        core_supervisor.tell(GetClusterCoreRef, sender);
        Ok(Behavior::same())
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("GetClusterCoreRefResp({_0})")]
pub(crate) struct GetClusterCoreRefResp(pub(crate) ActorRef);
