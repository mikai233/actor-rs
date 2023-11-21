use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use crate::actor::{Actor, Message};
use crate::actor::context::ActorContext;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub(crate) struct MemberJoin {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberJoin {
    type T = ClusterDaemon;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        todo!()
    }
}