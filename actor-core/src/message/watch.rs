use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Message, derive_more::Display)]
#[display("Watch {{ watchee: {}, watcher: {} }}", watchee, watcher)]
pub struct Watch {
    pub watchee: ActorRef,
    pub watcher: ActorRef,
}