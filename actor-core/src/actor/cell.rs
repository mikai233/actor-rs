use dashmap::DashMap;

use crate::actor::actor_ref::ActorRef;
use crate::actor::fault_handing::ChildRestartStats;
use crate::cell::ActorCell;

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &DashMap<String, ChildRestartStats>;
    fn get_single_child(&self, name: &String) -> Option<ActorRef>;
}