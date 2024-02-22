use crate::actor::actor_ref::ActorRef;
use crate::DynMessage;

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) message: DynMessage,
    pub(crate) sender: Option<ActorRef>,
}

impl Envelope {
    pub fn new(message: DynMessage, sender: Option<ActorRef>) -> Self {
        Self {
            message,
            sender,
        }
    }

    pub fn name(&self) -> &str {
        self.message.name()
    }
}