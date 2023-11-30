use std::sync::atomic::AtomicUsize;

use crate::DynMessage;
use crate::routing::router::{Routee, RoutingLogic};

#[derive(Debug)]
pub struct RoundRobinRoutingLogic {
    next: AtomicUsize,
}

impl RoutingLogic for RoundRobinRoutingLogic {
    fn select(&self, _message: &DynMessage, routees: &Vec<Box<dyn Routee>>) -> &Box<dyn Routee> {
        // if !routees.is_empty() {
        //     let size = routees.len();
        //     let index = self.next.fetch_add(1, Ordering::Relaxed) % size;
        //     routees[index].clone()
        // } else {
        //     Box::new(NoRoutee)
        // }
        todo!()
    }
}