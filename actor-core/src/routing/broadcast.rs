use std::sync::Arc;

use crate::DynMessage;
use crate::routing::router::{MaybeRef, Routee, RoutingLogic, SeveralRoutees};

#[derive(Debug, Clone)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Arc<Box<dyn Routee>>>) -> MaybeRef<'a, Box<dyn Routee>> {
        MaybeRef::Own(Box::new(SeveralRoutees { routees: routees.clone() }))
    }
}
