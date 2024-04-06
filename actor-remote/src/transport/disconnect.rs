use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::transport::connection_status::ConnectionStatus;
use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnect {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if let Some(ConnectionStatus::Connecting(handle)) = actor.connections.remove(&self.addr) {
            handle.abort();
        }
        actor.message_buffer.remove(&self.addr);
        info!("{} disconnect from {}", context.myself(), self.addr);
        Ok(())
    }
}