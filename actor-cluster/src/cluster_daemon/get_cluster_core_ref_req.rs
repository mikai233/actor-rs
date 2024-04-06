use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, ContextExt};
use actor_core::actor_ref::ActorRef;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, EmptyCodec)]
pub(crate) struct GetClusterCoreRefReq;

#[async_trait]
impl Message for GetClusterCoreRefReq {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if actor.core_supervisor.is_none() {
            actor.create_children(context)?;
        }
        actor.core_supervisor.foreach(|core_supervisor| {
            let msg = crate::cluster_core_supervisor::get_cluster_core_ref::GetClusterCoreRef;
            context.forward(core_supervisor, msg.into_dyn());
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct GetClusterCoreRefResp(pub(crate) ActorRef);