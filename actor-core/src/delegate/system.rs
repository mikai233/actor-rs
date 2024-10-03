use std::any::{Any, type_name};

use async_trait::async_trait;

use crate::{Actor, CodecMessage, DynMessage, MessageType, SystemMessage};
use crate::actor::context::ActorContext;
use crate::delegate::downcast_box_message;
use crate::message::codec::MessageRegistry;
use crate::message::MessageDecoder;

pub struct SystemDelegate {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn SystemMessage>,
}

impl SystemDelegate where {
    pub fn new<M>(message: M) -> Self where M: SystemMessage {
        Self {
            name: type_name::<M>(),
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

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(self: Box<Self>, reg: &MessageRegistry) -> anyhow::Result<Vec<u8>> {
        self.message.encode(reg)
    }

    fn clone_box(&self) -> anyhow::Result<Box<dyn CodecMessage>> {
        let message = self.message.clone_box()?.into_codec();
        Ok(message)
    }

    fn cloneable(&self) -> bool {
        self.message.cloneable()
    }

    fn into_dyn(self) -> DynMessage {
        let Self { name, message } = self;
        DynMessage { name, ty: MessageType::System, message: message.into_codec() }
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
            ty: MessageType::System,
            message: Box::new(self),
        }
    }
}
