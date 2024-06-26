use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::transport::TransportActor;

#[derive(EmptyCodec)]
pub(super) struct SpawnInbound {
    pub(super) peer_addr: SocketAddr,
    pub(super) fut: Pin<Box<dyn Future<Output=()> + Send>>,
}

#[async_trait]
impl Message for SpawnInbound {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.spawn_fut(format!("connection_in_{}", self.peer_addr), self.fut)?;
        Ok(())
    }
}