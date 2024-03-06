use dashmap::DashMap;

use crate::actor_ref::ActorRef;
use crate::cell::actor_cell::ActorCell;

pub mod envelope;
pub(crate) mod runtime;
pub(crate) mod actor_cell;

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState>;
    fn get_single_child(&self, name: &str) -> Option<ActorRef>;
}
