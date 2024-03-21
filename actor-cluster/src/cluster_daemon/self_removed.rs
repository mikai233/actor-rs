use std::collections::HashMap;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct SelfRemoved;

#[async_trait]
impl Message for SelfRemoved {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let cluster = actor.cluster.as_result_mut()?;
        *cluster.members_write() = HashMap::new();
        let mut self_member = cluster.self_member_write();
        self_member.status = MemberStatus::Removed;
        info!("{} self removed", self_member);
        Ok(())
    }
}