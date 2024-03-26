use std::ops::Deref;

use crate::actor_ref::ActorRef;

#[derive(Debug, Clone)]
pub struct Terminated {
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}

impl Terminated {
    pub fn new(watchee: ActorRef) -> Self {
        Self {
            actor: watchee,
            existence_confirmed: true,
            address_terminated: false,
        }
    }
}

impl Deref for Terminated {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.actor
    }
}