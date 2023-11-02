use std::any::Any;

use crate::actor::Message;
use crate::actor_ref::ActorRef;
use crate::message::ActorMessage;

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) message: ActorMessage,
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
        name: &'static str,
        message: Vec<u8>,
    },
    Unkonwn(Box<dyn Any + Send + 'static>),
}
