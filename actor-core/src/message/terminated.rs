use crate::actor_ref::ActorRef;
use actor_derive::Message;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

#[derive(Debug, Clone, Message)]
#[cloneable]
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

impl Display for Terminated {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Terminated {{actor: {}, existence_confirmed: {}, address_terminated: {} }}",
            self.actor,
            self.existence_confirmed,
            self.address_terminated,
        )
    }
}