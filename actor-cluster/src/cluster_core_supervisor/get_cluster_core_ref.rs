use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::cluster_core_supervisor::ClusterCoreSupervisor;
use crate::cluster_daemon::get_cluster_core_ref_req::GetClusterCoreRefResp;

#[derive(Debug, EmptyCodec)]
pub(crate) struct GetClusterCoreRef;

#[async_trait]
impl Message for GetClusterCoreRef {
    type A = ClusterCoreSupervisor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let core_daemon = match &actor.core_daemon {
            None => {
                actor.create_children(context)?
            }
            Some(core_daemon) => {
                core_daemon.clone()
            }
        };
        let sender = context.sender().into_result()?;
        sender.cast_orphan_ns(GetClusterCoreRefResp(core_daemon));
        Ok(())
    }
}