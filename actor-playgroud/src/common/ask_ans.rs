use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::{EmptyTestActor, Message};
use actor_core::{MessageCodec, OrphanCodec};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;

#[derive(Encode, Decode, MessageCodec)]
pub struct MessageToAsk;

#[async_trait]
impl Message for MessageToAsk {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.sender().unwrap().cast_orphan_ns(MessageToAns {
            content: "hello world".to_string(),
        });
        Ok(())
    }
}

#[derive(Encode, Decode, OrphanCodec)]
pub struct MessageToAns {
    pub content: String,
}