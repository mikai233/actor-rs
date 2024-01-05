use std::net::SocketAddr;
use bincode::{Decode, Encode};
use actor_core::actor::context::ActorContext;


use actor_derive::MessageCodec;

use actor_core::Message;
use crate::cluster::daemon::cluster_daemon::ClusterDaemon;

#[derive(Debug,  Encode,Decode, MessageCodec)]
pub struct MemberLeave {
    pub addr: SocketAddr,
}

impl Message for MemberLeave {
    type A = ClusterDaemon;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}