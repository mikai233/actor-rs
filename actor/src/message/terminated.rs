use crate::actor::Message;
use crate::actor_ref::ActorRef;
use crate::system::ActorSystem;

pub trait WatchTerminated: Message {
    fn watch_actor(&self, system: &ActorSystem) -> ActorRef;
}