use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::routing::routee::TRoutee;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NoRoutee;

impl TRoutee for NoRoutee {
    fn send(&self, _: DynMessage, _: Option<ActorRef>) {}
}
