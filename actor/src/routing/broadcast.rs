use crate::DynMessage;
use crate::routing::router::{Routee, RoutingLogic};

#[derive(Debug)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select(&self, _message: &DynMessage, routees: &Vec<Box<dyn Routee>>) -> &Box<dyn Routee> {
        todo!()
        // Box::new(SeveralRoutees { routees: routees.clone() })
    }
}
