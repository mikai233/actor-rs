use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::DynMessage;
use crate::routing::router::{NoRoutee, Routee, RoutingLogic};

#[derive(Debug)]
pub struct RoundRobinRoutingLogic {
    next: AtomicUsize,
}

impl RoutingLogic for RoundRobinRoutingLogic {
    fn select(&self, _message: &DynMessage, routees: &Vec<Arc<Box<dyn Routee>>>) -> Arc<Box<dyn Routee>> {
        if !routees.is_empty() {
            let size = routees.len();
            let index = self.next.fetch_add(1, Ordering::Relaxed) % size;
            routees[index].clone()
        } else {
            Arc::new(Box::new(NoRoutee))
        }
    }
}