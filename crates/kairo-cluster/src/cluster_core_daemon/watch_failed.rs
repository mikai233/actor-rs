use async_trait::async_trait;

use kairo_core::actor::context::ActorContext;
use kairo_core::{EmptyCodec, Message};

use crate::cluster_core_daemon::ClusterCoreDaemon;

#[derive(Debug, EmptyCodec)]
pub(crate) struct WatchFailed;

#[async_trait]
impl Message for WatchFailed {
    type A = ClusterCoreDaemon;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        actor.watch_cluster_members();
        Ok(())
    }
}
