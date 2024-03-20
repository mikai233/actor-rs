use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_core_daemon::ClusterCoreDaemon;

#[derive(Debug, EmptyCodec)]
pub(super) struct ExitingCompletedReq;

#[async_trait]
impl Message for ExitingCompletedReq {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("Exiting completed");
        todo!()
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(super) struct ExitingCompletedResp;