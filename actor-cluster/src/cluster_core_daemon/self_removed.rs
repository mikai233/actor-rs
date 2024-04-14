use ahash::{HashMap, HashMapExt};
use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(crate) struct SelfRemoved;

#[async_trait]
impl Message for SelfRemoved {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        *actor.cluster.members_write() = HashMap::new();
        let mut self_member = actor.cluster.self_member_write();
        self_member.status = MemberStatus::Removed;
        info!("{} self removed", self_member);
        Ok(())
    }
}