use std::any::Any;
use std::borrow::Cow;

use crate::actor::{DynamicMessage, Message};
use crate::actor_ref::ActorRef;
use crate::message::ActorMessage;

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) message: DynamicMessage,
    pub(crate) sender: Option<ActorRef>,
}

impl Envelope {
    pub fn name(&self) -> &str {
        self.message.name()
    }
}

#[derive(Debug)]
pub enum UserEnvelope<M>
    where
        M: Message,
{
    Local(M),
    Remote {
        name: Cow<'static, str>,
        message: Vec<u8>,
    },
    Unknown {
        name: &'static str,
        message: Box<dyn Any + Send + 'static>,
    },
}
