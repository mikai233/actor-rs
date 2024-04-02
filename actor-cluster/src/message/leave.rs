use std::net::SocketAddr;

use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::cluster_daemon::ClusterDaemon;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub struct MemberLeave {
    pub addr: SocketAddr,
}

#[async_trait]
impl Message for MemberLeave {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        todo!()
    }
}