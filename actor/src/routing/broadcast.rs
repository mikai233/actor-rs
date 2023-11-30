use std::sync::Arc;

use crate::DynMessage;
use crate::routing::router::{Routee, RoutingLogic, SeveralRoutees};

#[derive(Debug)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select(&self, _message: &DynMessage, routees: &Vec<Arc<Box<dyn Routee>>>) -> Arc<Box<dyn Routee>> {
        Arc::new(Box::new(SeveralRoutees { routees: routees.clone() }))
    }
}
