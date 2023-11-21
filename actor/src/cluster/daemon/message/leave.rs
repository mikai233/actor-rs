use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use actor_derive::MessageCodec;

use crate::actor::Actor;
use crate::actor::context::ActorContext;
use crate::actor::Message;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;

#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(ClusterDaemon)]
pub(crate) struct MemberLeave {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberLeave {
    type T = ClusterDaemon;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        todo!()
    }
}