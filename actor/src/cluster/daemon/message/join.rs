use std::net::SocketAddr;

use actor_derive::EmptyCodec;

use crate::actor::{Actor, Message};
use crate::actor::context::ActorContext;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;

#[derive(Debug, EmptyCodec)]
pub(crate) struct MemberJoin {
    pub(crate) addr: SocketAddr,
}

impl Message for MemberJoin {
    type T = ClusterDaemon;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        todo!()
    }
}