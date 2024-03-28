use std::fmt::{Display, Formatter};
use std::ops::Deref;

use crate::actor_ref::ActorRef;
use crate::ext::type_name_of;

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

impl Display for Terminated {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {{actor: {}, existence_confirmed: {}, address_terminated: {} }}",
            type_name_of::<Self>(),
            self.actor,
            self.existence_confirmed,
            self.address_terminated,
        )
    }
}