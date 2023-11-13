use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::actor::{CodecMessage, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::SerializedActorRef;
use crate::decoder::MessageDecoder;
use crate::ext::encode_bytes;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::system_message_decoder;

#[derive(Serialize, Deserialize)]
pub(crate) struct DeathWatchNotification(pub(crate) SerializedActorRef);

impl CodecMessage for DeathWatchNotification {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        Some(system_message_decoder!(DeathWatchNotification))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        Some(encode_bytes(self))
    }
}

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        let actor_ref = context.system().provider().resolve_actor_ref(&self.0.path);
        context.watched_actor_terminated(actor_ref);
        Ok(())
    }
}