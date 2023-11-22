use crate::actor_ref::ActorRef;
use crate::DynamicMessage;

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