use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use ahash::{HashMap, HashMapExt};
use tracing::info;

use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::member::MemberStatus;

#[derive(Debug, Message, derive_more::Display)]
#[display("SelfRemoved")]
pub(crate) struct SelfRemoved;

impl MessageHandler<ClusterCoreDaemon> for SelfRemoved {
    fn handle(
        actor: &mut ClusterCoreDaemon,
        ctx: &mut <ClusterCoreDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterCoreDaemon>,
    ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
        *actor.cluster.members_write() = HashMap::new();
        let mut self_member = actor.cluster.self_member_write();
        self_member.status = MemberStatus::Removed;
        info!("{} self removed", self_member);
        Ok(Behavior::same())
    }
}
