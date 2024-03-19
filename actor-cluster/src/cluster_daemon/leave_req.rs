use async_trait::async_trait;
use tracing::instrument;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, EmptyCodec)]
pub(super) struct LeaveReq;

#[async_trait]
impl Message for LeaveReq {
    type A = ClusterDaemon;

    #[instrument(skip(context, actor))]
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let reply_to = context.sender().into_result()?.clone();
        let cluster = actor.cluster.as_result()?;
        cluster.leave(cluster.self_address().clone());
        actor.reply_to = Some(reply_to);
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(super) struct LeaveResp;