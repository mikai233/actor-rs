use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_core::message::message_buffer::BufferEnvelope;
use actor_derive::EmptyCodec;

use crate::net::tcp_transport::connection::ConnectionTx;
use crate::net::tcp_transport::connection_status::ConnectionStatus;
use crate::net::tcp_transport::TcpTransportActor;

#[derive(EmptyCodec)]
pub(super) struct Connected {
    pub(super) addr: SocketAddr,
    pub(super) tx: ConnectionTx,
}

#[async_trait]
impl Message for Connected {
    type A = TcpTransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
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