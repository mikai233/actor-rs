use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::info;

use kairo_core::CMessageCodec;
use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::{EmptyTestActor, Message};

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub struct TestMessage;

#[async_trait]
impl Message for TestMessage {
    type A = EmptyTestActor;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        info!("{} recv {:?}", context.myself(), self);
        Ok(())
    }
}
