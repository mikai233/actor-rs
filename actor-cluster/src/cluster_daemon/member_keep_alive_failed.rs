use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::etcd_actor::keep_alive::KeepAliveFailed;

#[derive(Debug, EmptyCodec)]
pub(super) struct MemberKeepAliveFailed(pub(super) Option<KeepAliveFailed>);

#[async_trait]
impl Message for MemberKeepAliveFailed {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("cluster member {} keep alive failed, retry it", context.myself());
        actor.try_keep_alive(context).await;
        Ok(())
    }
}