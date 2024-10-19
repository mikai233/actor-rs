use actor_core::message::handler::MessageHandler;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, Message, derive_more::Display)]
#[display("ExitingCompletedReq")]
pub(super) struct ExitingCompletedReq;

impl MessageHandler<ClusterCoreDaemon> for ExitingCompletedReq {
    fn handle(
        actor: &mut ClusterCoreDaemon,
        ctx: &mut <ClusterCoreDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterCoreDaemon>,
    ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
        let sender = context.sender().into_result()?;
        info!("Exiting completed");
        actor.exiting_tasks_in_progress = false;
        let mut self_member = actor.cluster.self_member().clone();
        self_member.status = MemberStatus::Removed;
        actor.update_member_to_etcd(&self_member).await?;
        sender.cast_orphan_ns(ExitingCompletedResp);
        actor.cluster.shutdown()?;
        todo!()
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("ExitingCompletedResp")]
pub(super) struct ExitingCompletedResp;
