use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::net::tcp_transport::connection_status::ConnectionStatus;
use crate::net::tcp_transport::TcpTransportActor;

#[derive(EmptyCodec)]
pub(super) struct Disconnect {
    pub(super) addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnect {
    type A = TcpTransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(ConnectionStatus::Connecting(handle)) = actor.connections.remove(&self.addr) {
            handle.abort();
        }
        actor.message_buffer.remove(&self.addr);
        info!("{} disconnect from {}", context.myself(), self.addr);
        Ok(())
    }
}