use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::EmptyCodec;
use actor_core::Message;
use actor_core::message::message_buffer::BufferEnvelope;

use crate::artery::ArteryActor;
use crate::artery::connection::ConnectionTx;
use crate::artery::connection_status::ConnectionStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct Connected {
    pub(super) addr: SocketAddr,
    pub(super) tx: ConnectionTx,
}

#[async_trait]
impl Message for Connected {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
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