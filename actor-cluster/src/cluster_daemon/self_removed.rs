use std::collections::HashMap;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_daemon::leave_req::LeaveResp;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct SelfRemoved;

#[async_trait]
impl Message for SelfRemoved {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let cluster = actor.cluster.as_result_mut()?;
        *cluster.members_write() = HashMap::new();
        let mut self_member = cluster.self_member_write();
        self_member.status = MemberStatus::Removed;
        if let Some(reply_to) = actor.reply_to.take() {
            reply_to.resp(LeaveResp);
        }
        info!("{} self removed", self_member);
        context.stop(context.myself());
        Ok(())
    }
}