use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::EmptyCodec;
use actor_core::Message;
use actor_core::actor::context::{ActorContext, Context};

use crate::transport::TransportActor;
use crate::transport::connection_status::ConnectionStatus;

#[derive(Debug, EmptyCodec)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnect {
    type A = TransportActor;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        if let Some(ConnectionStatus::Connecting(handle)) = actor.connections.remove(&self.addr) {
            handle.abort();
        }
        actor.message_buffer.remove(&self.addr);
        info!("{} disconnect from {}", context.myself(), self.addr);
        Ok(())
    }
}
