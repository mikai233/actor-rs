use std::any::Any;
use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::actor::{CodecMessage, SystemMessage};
use crate::actor::context::{ActorContext, Context};
use crate::decoder::MessageDecoder;
use crate::ext::encode_bytes;
use crate::provider::ActorRefFactory;
use crate::system_message_decoder;

#[derive(Debug, Serialize, Deserialize)]
pub struct PoisonPill;

impl CodecMessage for PoisonPill {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        Some(system_message_decoder!(PoisonPill))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        Some(encode_bytes(self))
    }
}

#[async_trait]
impl SystemMessage for PoisonPill {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} receive PoisonPill", context.myself());
        context.stop(context.myself());
        Ok(())
    }
}