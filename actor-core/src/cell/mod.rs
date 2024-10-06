use crate::actor::actor_system::ActorSystem;
use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use dashmap::DashMap;

pub mod envelope;
pub(crate) mod runtime;
pub(crate) mod actor_cell;

pub(crate) trait Cell {
    fn myself(&self) -> &ActorRef;

    fn system(&self) -> &ActorSystem;

    fn start(&self);

    fn suspend(&self);

    fn resume(&self, error: anyhow::Error);

    fn stop(&self);

    fn parent(&self) -> Option<&ActorRef>;

    fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState>;

    fn get_child_by_name(&self, name: &str) -> Option<&ActorRef>;

    fn get_single_child(&self, name: &str) -> Option<ActorRef>;

    fn send_message(&self, message: DynMessage, sender: Option<ActorRef>);

    fn send_system_message(&self, message: DynMessage, sender: Option<ActorRef>);
}
