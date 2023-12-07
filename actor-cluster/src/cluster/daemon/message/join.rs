use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use actor_core::Message;
use actor_cluster::::daemon::cluster_daemon::ClusterDaemon;
use actor_core::context::ActorContext;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub struct MemberJoin {
    pub addr: SocketAddr,
}

impl Message for MemberJoin {
    type A = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}