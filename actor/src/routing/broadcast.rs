use crate::DynMessage;
use crate::routing::router::{Routee, RoutingLogic, SeveralRoutees};

#[derive(Debug)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select(&self, _message: DynMessage, routees: &Vec<Box<dyn Routee>>) -> Box<dyn Routee> {
        Box::new(SeveralRoutees { routees: routees.clone() })
    }
}
