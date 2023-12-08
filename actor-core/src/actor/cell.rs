use std::collections::BTreeMap;
use std::sync::RwLock;
use crate::actor::actor_ref::ActorRef;
use crate::cell::ActorCell;

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>>;
    fn get_single_child(&self, name: &String) -> Option<ActorRef>;
}