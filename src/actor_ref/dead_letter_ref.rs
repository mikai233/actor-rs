use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tracing::info;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::ActorMessage;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct DeadLetterActorRef {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
}

impl TActorRef for DeadLetterActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        let name = message.name();
        match sender {
            None => {
                info!("dead letter recv message {}", name);
            }
            Some(sender) => {
                info!("dead letter recv message {} from {}", name, sender);
            }
        }
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child<I>(&self, _names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        None
    }
}
