use crate::actor_ref::ActorRef;
use crate::routing::routee::TRoutee;
use crate::DynMessage;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NoRoutee;

impl TRoutee for NoRoutee {
    fn send(&self, _message: DynMessage, _sender: Option<ActorRef>) {}
}
