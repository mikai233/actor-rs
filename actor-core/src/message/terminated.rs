use crate::actor_ref::ActorRef;
use crate::Message;

pub trait WatchTerminated: Message {
    fn watch_actor(&self) -> &ActorRef;
}