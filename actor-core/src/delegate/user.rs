use std::any::Any;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;

use crate::{Actor, AsyncMessage, CodecMessage, DynMessage, Message, MessageType};
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;

pub struct UserDelegate<A> where A: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn Message<A=A>>,
}

impl<A> Debug for UserDelegate<A> where A: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserDelegate")
            .field("name", &self.name)
            .field("message", &"..")
            .finish()
    }
}

impl<A> UserDelegate<A> where A: Actor {
    pub fn new<M>(message: M) -> Self where M: Message<A=A> {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl<A> CodecMessage for UserDelegate<A> where A: 'static + Actor + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        self.message.encode()
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        self.message.dyn_clone()
    }
}

impl<A> Message for UserDelegate<A> where A: Actor + Send + 'static {
    type A = A;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        self.message.handle(context, actor)
    }
}

impl<A> Into<DynMessage> for UserDelegate<A> where A: Actor {
    fn into(self) -> DynMessage {
        DynMessage {
            name: self.name,
            message_type: MessageType::User,
            boxed: Box::new(self),
        }
    }
}

pub struct AsyncUserDelegate<A> where A: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn AsyncMessage<A=A>>,
}

impl<A> Debug for AsyncUserDelegate<A> where A: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncUserDelegate")
            .field("name", &self.name)
            .field("message", &"..")
            .finish()
    }
}

impl<A> AsyncUserDelegate<A> where A: Actor {
    pub fn new<M>(message: M) -> Self where M: AsyncMessage<A=A> {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl<A> CodecMessage for AsyncUserDelegate<A> where A: 'static + Actor + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        self.message.encode()
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        self.message.dyn_clone()
    }
}

#[async_trait]
impl<A> AsyncMessage for AsyncUserDelegate<A> where A: Actor + Send + 'static {
    type A = A;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        self.message.handle(context, actor).await
    }
}

impl<A> Into<DynMessage> for AsyncUserDelegate<A> where A: Actor {
    fn into(self) -> DynMessage {
        DynMessage {
            name: self.name,
            message_type: MessageType::AsyncUser,
            boxed: Box::new(self),
        }
    }
}