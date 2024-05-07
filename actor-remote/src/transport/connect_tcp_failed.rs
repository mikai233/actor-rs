use std::net::SocketAddr;

use async_trait::async_trait;

use actor_core::{EmptyCodec, Message};
use actor_core::actor::context::ActorContext;

use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct ConnectFailed {
    pub(super) addr: SocketAddr,
}

#[async_trait]
impl Message for ConnectFailed {
    type A = TransportActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        actor.connections.remove(&self.addr);
        Ok(())
    }
}