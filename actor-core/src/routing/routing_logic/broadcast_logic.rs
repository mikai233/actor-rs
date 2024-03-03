use crate::DynMessage;
use crate::ext::maybe_ref::MaybeRef;
use crate::routing::routee::{Routee, SeveralRoutees};
use crate::routing::routing_logic::RoutingLogic;

#[derive(Debug, Clone)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Box<dyn Routee>>) -> MaybeRef<'a, Box<dyn Routee>> {
        MaybeRef::Own(Box::new(SeveralRoutees { routees: routees.clone() }))
    }
}
