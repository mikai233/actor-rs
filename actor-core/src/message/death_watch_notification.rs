use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[cloneable]
pub struct DeathWatchNotification {
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}

impl Display for DeathWatchNotification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeathWatchNotification {{ actor: {}, existence_confirmed: {}, address_terminated: {} }}",
            self.actor,
            self.existence_confirmed,
            self.address_terminated,
        )
    }
}