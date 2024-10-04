use crate::actor_ref::ActorRef;
use crate::message::{DynMessage, Message};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) message: DynMessage,
    pub(crate) sender: Option<ActorRef>,
}

impl Envelope {
    pub fn new<M>(message: M, sender: Option<ActorRef>) -> Self
    where
        M: Message + 'static,
    {
        Self {
            message: Box::new(message),
            sender,
        }
    }

    pub fn name(&self) -> &'static str {
        self.message.signature().name
    }
}

impl Display for Envelope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Envelope {{ message: {}, sender: {:?} }}", self.name(), self.sender)
    }
}