use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use crate::{Actor, Message};
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;
use crate::context::ActorContext;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub(crate) struct MemberJoin {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberJoin {
    type T = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        todo!()
    }
}