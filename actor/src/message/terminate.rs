use std::any::Any;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::actor::context::ActorContext;
use crate::actor::{CodecMessage, SystemMessage};
use crate::decoder::MessageDecoder;
use crate::ext::encode_bytes;
use crate::system_message_decoder;

#[derive(Serialize, Deserialize)]
pub(crate) struct Terminate;

impl CodecMessage for Terminate {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        Some(system_message_decoder!(Terminate))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        Some(encode_bytes(self))
    }
}

#[async_trait(? Send)]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        context.handle_terminate();
        Ok(())
    }
}