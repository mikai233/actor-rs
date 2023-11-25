use crate::actor_ref::ActorRef;
use crate::Message;
use crate::system::ActorSystem;

pub trait WatchTerminated: Message {
    fn watch_actor(&self) -> &ActorRef;
}