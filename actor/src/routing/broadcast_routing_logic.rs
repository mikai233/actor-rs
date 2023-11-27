use crate::DynamicMessage;
use crate::routing::router::{Routee, RoutingLogic};

#[derive(Copy, Clone)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select(&self, message: DynamicMessage, routees: &Vec<Box<dyn Routee>>) -> &Box<dyn Routee> {
        todo!()
    }
}
