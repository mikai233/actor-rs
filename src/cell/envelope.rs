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