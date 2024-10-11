use crate::ext::maybe_ref::MaybeRef;
use crate::message::DynMessage;
use crate::routing::routee::Routee;

pub mod broadcast_routing_logic;
pub mod round_robin_routing_logic;

pub trait RoutingLogic: Send {
    fn select<'a>(&self, message: &DynMessage, routees: &'a Vec<Routee>) -> MaybeRef<'a, Routee>;
}
