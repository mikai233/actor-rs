use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use crate::Message;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;
use crate::context::ActorContext;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub(crate) struct MemberJoin {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberJoin {
    type A = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}