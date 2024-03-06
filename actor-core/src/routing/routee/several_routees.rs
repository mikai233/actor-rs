use std::ops::Deref;

use tracing::error;

use crate::actor_ref::ActorRef;
use crate::DynMessage;
use crate::routing::routee::{Routee, TRoutee};

//TODO reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeveralRoutees {
    pub routees: Vec<Routee>,
}

impl Deref for SeveralRoutees {
    type Target = Vec<Routee>;

    fn deref(&self) -> &Self::Target {
        &self.routees
    }
}

impl TRoutee for SeveralRoutees {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        for routee in &self.routees {
            match message.dyn_clone() {
                Ok(message) => {
                    routee.send(message, sender.clone());
                }
                Err(_) => {
                    error!("route message {} not impl dyn_clone", message.name())
                }
            }
        }
    }
}