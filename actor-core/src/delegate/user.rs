use std::any::{Any, type_name};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use async_trait::async_trait;

use crate::{Actor, CodecMessage, DynMessage, Message, MessageType};
use crate::actor::context::ActorContext;
use crate::delegate::downcast_box_message;
use crate::message::message_registry::MessageRegistry;
use crate::message::MessageDecoder;

pub struct UserDelegate<A> where A: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn Message<A=A>>,
    _phantom: PhantomData<A>,
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
            name: type_name::<M>(),
            message: Box::new(message),
            _phantom: Default::default(),
        }
    }

    pub fn downcast<M>(self) -> eyre::Result<M> where M: Message {
        let Self { name, message, .. } = self;
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

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(self: Box<Self>, message_registration: &MessageRegistry) -> eyre::Result<Vec<u8>> {
        self.message.encode(message_registration)
    }

    fn clone_box(&self) -> eyre::Result<Box<dyn CodecMessage>> {
        let message = self.message.clone_box()?.into_codec();
        Ok(message)
    }

    fn cloneable(&self) -> bool {
        self.message.cloneable()
    }

    fn into_dyn(self) -> DynMessage {
        let Self { name, message, .. } = self;
        DynMessage { name, ty: MessageType::User, message: message.into_codec() }
    }
}

#[async_trait]
impl<A> Message for UserDelegate<A> where A: Actor + Send + 'static {
    type A = A;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        self.message.handle(context, actor).await
    }
}