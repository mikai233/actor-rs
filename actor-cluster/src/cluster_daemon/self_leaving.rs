use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct SelfLeaving;

#[async_trait]
impl Message for SelfLeaving {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut member = actor.cluster.as_result()?.self_member().clone();
        member.status = MemberStatus::Removed;
        actor.update_member_to_etcd(&member).await?;
        Ok(())
    }
}