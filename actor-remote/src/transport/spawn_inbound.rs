use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::transport::TransportActor;

#[derive(EmptyCodec)]
pub(super) struct SpawnInbound {
    pub(super) fut: Pin<Box<dyn Future<Output=()> + Send>>,
}

#[async_trait]
impl Message for SpawnInbound {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        context.spawn_fut(self.fut);
        Ok(())
    }
}