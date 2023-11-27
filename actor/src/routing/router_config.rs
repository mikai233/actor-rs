use dyn_clone::DynClone;

use crate::routing::router::Router;
use crate::routing::router_actor::RouterActor;
use crate::system::ActorSystem;

pub trait RouterConfig: DynClone {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
}

dyn_clone::clone_trait_object!(RouterConfig);