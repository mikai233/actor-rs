use async_trait::async_trait;

use actor_core::actor::address::Address;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(crate) struct LeaveCluster(pub(crate) Address);

#[async_trait]
impl Message for LeaveCluster {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let member = {
            actor.cluster.as_mut()
                .unwrap()
                .members()
                .iter()
                .find(|(_, m)| m.addr.address == self.0)
                .map(|(_, m)| m)
                .cloned()
        };
        if let Some(mut member) = member {
            member.status = MemberStatus::Leaving;
            actor.update_member_to_etcd(&member).await?;
        }
        Ok(())
    }
}