use std::ops::Deref;

use crate::actor::actor_selection::ActorSelection;
use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::routing::routee::TRoutee;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ActorSelectionRoutee(pub ActorSelection);

impl Deref for ActorSelectionRoutee {
    type Target = ActorSelection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TRoutee for ActorSelectionRoutee {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.tell(message, sender);
    }
}
