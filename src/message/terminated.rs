use std::any::Any;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::actor::context::ActorContext;
use crate::actor::{CodecMessage, SystemMessage};
use crate::actor_ref::SerializedActorRef;
use crate::decoder::MessageDecoder;
use crate::ext::encode_bytes;
use crate::system_message_decoder;

#[derive(Serialize, Deserialize)]
pub(crate) struct Terminated(SerializedActorRef);

impl CodecMessage for Terminated {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        Some(system_message_decoder!(Terminated))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        Some(encode_bytes(self))
    }
}

#[async_trait(? Send)]
impl SystemMessage for Terminated {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}