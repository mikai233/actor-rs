use actor_core::actor::address::Address;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, Message, derive_more::Display, derive_more::Deref)]
#[display("Leave({_0})")]
pub(crate) struct Leave(pub(crate) Address);

impl MessageHandler<ClusterCoreDaemon> for Leave {
    fn handle(
        actor: &mut ClusterCoreDaemon,
        ctx: &mut <ClusterCoreDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterCoreDaemon>,
    ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
        let member = {
            actor
                .cluster
                .members()
                .iter()
                .find(|(_, m)| m.unique_address.address == message)
                .map(|(_, m)| m)
                .cloned()
        };
        if let Some(mut member) = member {
            member.status = MemberStatus::Leaving;
            // actor.update_member_to_etcd(&member).await?;
        }
        Ok(Behavior::same())
    }
}
