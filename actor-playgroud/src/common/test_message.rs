use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::info;

use actor_core::{EmptyTestActor, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::CMessageCodec;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub struct TestMessage;

#[async_trait]
impl Message for TestMessage {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        info!("{} recv {:?}", context.myself(), self);
        Ok(())
    }
}