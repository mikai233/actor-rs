use async_trait::async_trait;

use actor_core::actor::address::Address;
use actor_core::actor::context::Context;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(crate) struct Leave(pub(crate) Address);

#[async_trait]
impl Message for Leave {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, _context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let member = {
            actor.cluster.members()
                .iter()
                .find(|(_, m)| m.unique_address.address == self.0)
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