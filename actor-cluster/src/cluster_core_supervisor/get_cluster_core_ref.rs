use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_core_supervisor::ClusterCoreSupervisor;

#[derive(Debug, EmptyCodec)]
pub(crate) struct GetClusterCoreRefReq;

#[async_trait]
impl Message for GetClusterCoreRefReq {
    type A = ClusterCoreSupervisor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let core_daemon = match &actor.core_daemon {
            None => {
                actor.create_children(context)?
            }
            Some(core_daemon) => {
                core_daemon.clone()
            }
        };
        let sender = context.sender().into_result()?;
        sender.resp(GetClusterCoreRefResp(core_daemon));
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct GetClusterCoreRefResp(pub(crate) ActorRef);