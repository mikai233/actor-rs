use std::cell::Cell;

use crate::DynMessage;
use crate::ext::maybe_ref::MaybeRef;
use crate::routing::routee::{NoRoutee, Routee};
use crate::routing::routing_logic::RoutingLogic;

#[derive(Debug, Default)]
pub struct RoundRobinRoutingLogic {
    next: Cell<usize>,
}

impl RoutingLogic for RoundRobinRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Box<dyn Routee>>) -> MaybeRef<'a, Box<dyn Routee>> {
        if !routees.is_empty() {
            let size = routees.len();
            let current = self.next.get();
            let index = current.wrapping_add(1) % size;
            self.next.set(index);
            MaybeRef::Ref(&routees[current])
        } else {
            MaybeRef::Own(Box::new(NoRoutee))
        }
    }
}