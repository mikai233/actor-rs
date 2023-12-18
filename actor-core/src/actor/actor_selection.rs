use crate::actor::actor_ref::ActorRef;

pub struct ActorSelection {
    pub(crate) anchor: ActorRef,
    pub(crate) path: Vec<String>,
}