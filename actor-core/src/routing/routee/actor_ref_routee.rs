use std::ops::Deref;

use crate::actor_ref::ActorRef;
use crate::DynMessage;
use crate::routing::routee::TRoutee;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActorRefRoutee(pub ActorRef);

impl Deref for ActorRefRoutee {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TRoutee for ActorRefRoutee {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.tell(message, sender);
    }
}