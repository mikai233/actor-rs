use crate::DynMessage;
use crate::ext::maybe_ref::MaybeRef;
use crate::routing::routee::Routee;

pub mod round_robin_routing_logic;
pub mod broadcast_routing_logic;

pub trait RoutingLogic: Send {
    fn select<'a>(&self, message: &DynMessage, routees: &'a Vec<Routee>) -> MaybeRef<'a, Routee>;
}
