use crate::actor::Message;
use crate::actor_ref::ActorRef;
use crate::system::ActorSystem;

pub trait WatchTerminated: Message {
    fn actor(&self, system: &ActorSystem) -> ActorRef;
}