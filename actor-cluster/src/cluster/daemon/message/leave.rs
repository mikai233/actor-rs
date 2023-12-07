use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use actor_cluster::::daemon::cluster_daemon::ClusterDaemon;
use actor_core::context::ActorContext;
use actor_core::Message;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub struct MemberLeave {
    pub addr: SocketAddr,
}

impl Message for MemberLeave {
    type A = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}