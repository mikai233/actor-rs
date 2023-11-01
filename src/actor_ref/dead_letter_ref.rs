use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::ActorMessage;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct DeadLetterActorRef {}

impl TActorRef for DeadLetterActorRef {
    fn system(&self) -> ActorSystem {
        todo!()
    }

    fn path(&self) -> &ActorPath {
        todo!()
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        todo!()
    }

    fn start(&self) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        todo!()
    }

    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>> {
        todo!()
    }
}