use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{Context, ActorContext};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::artery::ArteryActor;
use crate::artery::connection_status::ConnectionStatus;

#[derive(Debug, EmptyCodec)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnect {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(ConnectionStatus::Connecting(handle)) = actor.connections.remove(&self.addr) {
            handle.abort();
        }
        actor.message_buffer.remove(&self.addr);
        info!("{} disconnect from {}", context.myself(), self.addr);
        Ok(())
    }
}