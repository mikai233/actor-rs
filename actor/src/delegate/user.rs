use std::any::Any;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;

use crate::{Actor, AsyncMessage, BoxedMessage, CodecMessage, DynamicMessage, Message};
use crate::context::ActorContext;
use crate::decoder::MessageDecoder;

pub struct UserDelegate<T> where T: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn Message<T=T>>,
}

impl<T> Debug for UserDelegate<T> where T: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserDelegate")
            .field("name", &self.name)
            .field("message", &"..")
            .finish()
    }
}

impl<T> UserDelegate<T> where T: Actor {
    pub fn new<M>(message: M) -> Self where M: Message<T=T> {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl<T> CodecMessage for UserDelegate<T> where T: 'static + Actor + Send {
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

impl<T> Message for UserDelegate<T> where T: Actor + Send + 'static {
    type T = T;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        self.message.handle(context, state)
    }
}

impl<T> Into<DynamicMessage> for UserDelegate<T> where T: Actor {
    fn into(self) -> DynamicMessage {
        DynamicMessage::User(BoxedMessage {
            name: self.name,
            inner: Box::new(self),
        })
    }
}

pub struct AsyncUserDelegate<T> where T: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn AsyncMessage<T=T>>,
}

impl<T> Debug for AsyncUserDelegate<T> where T: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncUserDelegate")
            .field("name", &self.name)
            .field("message", &"..")
            .finish()
    }
}

impl<T> AsyncUserDelegate<T> where T: Actor {
    pub fn new<M>(message: M) -> Self where M: AsyncMessage<T=T> {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl<T> CodecMessage for AsyncUserDelegate<T> where T: 'static + Actor + Send {
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
impl<T> AsyncMessage for AsyncUserDelegate<T> where T: Actor + Send + 'static {
    type T = T;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        self.message.handle(context, state).await
    }
}

impl<T> Into<DynamicMessage> for AsyncUserDelegate<T> where T: Actor {
    fn into(self) -> DynamicMessage {
        DynamicMessage::User(BoxedMessage {
            name: self.name,
            inner: Box::new(self),
        })
    }
}