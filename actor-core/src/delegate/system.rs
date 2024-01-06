use std::any::Any;

use async_trait::async_trait;
use bincode::error::EncodeError;

use crate::{Actor, CodecMessage, DynMessage, MessageType, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::delegate::downcast_box_message;
use crate::message::message_registration::MessageRegistration;

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

    pub fn downcast<M>(self) -> anyhow::Result<M> where M: CodecMessage {
        let Self { name, message } = self;
        downcast_box_message(name, message.into_any())
    }

    pub fn downcast_ref<M>(&self) -> Option<&M> where M: CodecMessage {
        let Self { message, .. } = self;
        message.as_any().downcast_ref()
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

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        self.message.encode(reg)
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
