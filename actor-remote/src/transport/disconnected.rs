use std::net::SocketAddr;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct Disconnected {
    pub(super) addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnected {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.message_buffer.remove(&self.addr);
        info!("{} disconnected from {}", context.myself(), self.addr);
        Ok(())
    }
}