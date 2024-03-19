use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, EmptyCodec)]
pub(super) struct LeaveReq;

#[async_trait]
impl Message for LeaveReq {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let cluster = actor.cluster.as_result()?;
        cluster.leave(cluster.self_address().clone());
        // cluster.subscribe_cluster_event(context.myself().clone(),)
        //TODO
        Ok(())
    }
}