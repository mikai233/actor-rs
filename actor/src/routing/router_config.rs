use dyn_clone::DynClone;

use crate::Actor;
use crate::context::ActorContext;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::RouterActor;
use crate::system::ActorSystem;

pub trait RouterConfig: DynClone {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
}

dyn_clone::clone_trait_object!(RouterConfig);

pub trait Pool {
    fn nr_of_instances(sys: &ActorSystem) -> usize;
    fn new_routee<T>(routee_props: Props<T>, context: ActorContext) -> Box<dyn Routee> where T: Actor;
}