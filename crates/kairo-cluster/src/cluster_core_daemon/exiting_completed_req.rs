use async_trait::async_trait;
use tracing::info;

use kairo_core::Message;
use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::actor_ref::ActorRefExt;
use kairo_core::ext::option_ext::OptionExt;
use kairo_core::{EmptyCodec, OrphanEmptyCodec};

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ExitingCompletedReq;

#[async_trait]
impl Message for ExitingCompletedReq {
    type A = ClusterCoreDaemon;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        let sender = context.sender().into_result()?;
        info!("Exiting completed");
        actor.exiting_tasks_in_progress = false;
        let mut self_member = actor.cluster.self_member().clone();
        self_member.status = MemberStatus::Removed;
        actor.update_member_to_etcd(&self_member).await?;
        sender.cast_orphan_ns(ExitingCompletedResp);
        actor.cluster.shutdown()?;
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub(super) struct ExitingCompletedResp;
