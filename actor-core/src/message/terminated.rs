use crate::actor_ref::ActorRef;
use crate::Message;

pub trait Terminated: Message {
    fn actor(&self) -> &ActorRef;

    //TODO address terminated
}