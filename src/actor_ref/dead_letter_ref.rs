use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::ActorMessage;

#[derive(Debug, Clone)]
pub struct DeadLetterActorRef {}

impl TActorRef for DeadLetterActorRef {
    fn path(&self) -> &ActorPath {
        todo!()
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        todo!()
    }
}