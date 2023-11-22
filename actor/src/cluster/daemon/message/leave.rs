use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use crate::Actor;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;
use crate::context::ActorContext;
use crate::Message;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub(crate) struct MemberLeave {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberLeave {
    type T = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        todo!()
    }
}