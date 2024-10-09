use std::net::SocketAddr;

use async_trait::async_trait;

use actor_core::{EmptyCodec, Message};
use actor_core::actor::context::Context;

use crate::artery::ArteryActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct ConnectFailed {
    pub(super) addr: SocketAddr,
}

#[async_trait]
impl Message for ConnectFailed {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, _context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.connections.remove(&self.addr);
        Ok(())
    }
}