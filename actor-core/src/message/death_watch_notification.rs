use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[cloneable]
pub struct DeathWatchNotification {
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}