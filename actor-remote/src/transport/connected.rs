use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_core::message::message_buffer::BufferEnvelope;
use actor_derive::EmptyCodec;

use crate::transport::connection::ConnectionTx;
use crate::transport::connection_status::ConnectionStatus;
use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct Connected {
    pub(super) addr: SocketAddr,
    pub(super) tx: ConnectionTx,
}

#[async_trait]
impl Message for Connected {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.connections.insert(self.addr, ConnectionStatus::Connected(self.tx));
        info!("{} connected to {}", context.myself(), self.addr);
        if let Some(buffers) = actor.message_buffer.remove(&self.addr) {
            for envelope in buffers {
                let (message, sender) = envelope.into_inner();
                context.myself().tell(message, sender);
            }
        }
        Ok(())
    }
}