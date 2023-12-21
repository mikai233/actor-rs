use dashmap::DashMap;

use crate::actor::actor_ref::ActorRef;
use crate::cell::ActorCell;

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState>;
    fn get_single_child(&self, name: &str) -> Option<ActorRef>;
}