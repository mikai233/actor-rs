use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::etcd_actor::keep_alive::KeepAliveFailed;

#[derive(Debug, EmptyCodec)]
pub(crate) struct MemberKeepAliveFailed(pub(crate) Option<KeepAliveFailed>);

#[async_trait]
impl Message for MemberKeepAliveFailed {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("cluster member {} keep alive failed, retry it", context.myself());
        actor.try_keep_alive(context).await;
        Ok(())
    }
}