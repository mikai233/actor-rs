use std::any::Any;

use async_trait::async_trait;

use crate::actor::{BoxedMessage, CodecMessage, DynamicMessage, SystemMessage};
use crate::actor::context::ActorContext;
use crate::decoder::MessageDecoder;

pub struct SystemDelegate {
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

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        self.message.encode()
    }
}

#[async_trait]
impl SystemMessage for SystemDelegate {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        self.message.handle(context).await
    }
}

impl Into<DynamicMessage> for SystemDelegate {
    fn into(self) -> DynamicMessage {
        DynamicMessage::System(BoxedMessage {
            name: self.name,
            inner: Box::new(self),
        })
    }
}
