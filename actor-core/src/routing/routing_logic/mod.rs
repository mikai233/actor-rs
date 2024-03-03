pub mod round_robin_routing_logic;
pub mod broadcast_logic;

use crate::DynMessage;
use crate::ext::maybe_ref::MaybeRef;
use crate::routing::routee::Routee;

pub trait RoutingLogic: Send {
    fn select<'a>(&self, message: &DynMessage, routees: &'a Vec<Box<dyn Routee>>) -> MaybeRef<'a, Box<dyn Routee>>;
}
