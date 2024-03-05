use crate::actor::actor_ref::ActorRef;
use crate::DynMessage;
use crate::routing::routee::TRoutee;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NoRoutee;

impl TRoutee for NoRoutee {
    fn send(&self, _message: DynMessage, _sender: Option<ActorRef>) {}
}