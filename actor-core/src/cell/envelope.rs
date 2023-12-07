use crate::actor::actor_ref::ActorRef;
use crate::DynMessage;

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) message: DynMessage,
    pub(crate) sender: Option<ActorRef>,
}

impl Envelope {
    pub fn name(&self) -> &str {
        self.message.name()
    }
}