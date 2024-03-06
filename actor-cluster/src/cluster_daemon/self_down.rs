use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, EmptyCodec)]
pub(super) struct SelfDown;

#[async_trait]
impl Message for SelfDown {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.self_down().await?;
        Ok(())
    }
}