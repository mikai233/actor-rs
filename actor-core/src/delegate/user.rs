use std::any::Any;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use bincode::error::EncodeError;

use crate::{Actor, CodecMessage, DynMessage, Message, MessageType};
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::delegate::downcast_box_message;
use crate::message::message_registration::MessageRegistration;

pub struct UserDelegate<A> where A: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn Message<A=A>>,
}

impl<A> Debug for UserDelegate<A> where A: Actor {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
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

    pub fn downcast<M>(self) -> anyhow::Result<M> where M: Message {
        let Self { name, message } = self;
        downcast_box_message(name, message.into_any())
    }

    pub fn downcast_ref<M>(&self) -> Option<&M> where M: Message {
        let Self { message, .. } = self;
        message.as_any().downcast_ref()
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

    fn encode(&self, message_registration: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        self.message.encode(message_registration)
    }

    fn dyn_clone(&self) -> anyhow::Result<DynMessage> {
        self.message.dyn_clone()
    }

    fn is_cloneable(&self) -> bool {
        self.message.is_cloneable()
    }
}

#[async_trait]
impl<A> Message for UserDelegate<A> where A: Actor + Send + 'static {
    type A = A;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        self.message.handle(context, actor).await
    }
}

impl<A> Into<DynMessage> for UserDelegate<A> where A: Actor {
    fn into(self) -> DynMessage {
        DynMessage {
            name: self.name,
            ty: MessageType::User,
            message: Box::new(self),
        }
    }
}