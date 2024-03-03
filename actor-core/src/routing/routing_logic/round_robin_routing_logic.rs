use crate::DynMessage;
use crate::ext::maybe_ref::MaybeRef;
use crate::routing::routee::{NoRoutee, Routee};
use crate::routing::routing_logic::RoutingLogic;

#[derive(Debug, Default)]
pub struct RoundRobinRoutingLogic {
    next: usize,
}

impl RoutingLogic for RoundRobinRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Box<dyn Routee>>) -> MaybeRef<'a, Box<dyn Routee>> {
        if !routees.is_empty() {
            let size = routees.len();
            let index = self.next.wrapping_add(1) % size;
            MaybeRef::Ref(&routees[index])
        } else {
            MaybeRef::Own(Box::new(NoRoutee))
        }
    }
}