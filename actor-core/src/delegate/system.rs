use std::any::Any;

use async_trait::async_trait;
use bincode::error::EncodeError;

use crate::{Actor, CodecMessage, DynMessage, MessageType, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;

pub(crate) struct SystemDelegate {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn SystemMessage>,
}

impl SystemDelegate where {
    pub fn new<M>(message: M) -> Self where M: SystemMessage {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl CodecMessage for SystemDelegate {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        self.message.encode()
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        self.message.dyn_clone()
    }
}

#[async_trait]
impl SystemMessage for SystemDelegate {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        self.message.handle(context, actor).await
    }
}

impl Into<DynMessage> for SystemDelegate {
    fn into(self) -> DynMessage {
        DynMessage {
            name: self.name,
            message_type: MessageType::System,
            boxed: Box::new(self),
        }
    }
}
