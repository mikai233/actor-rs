use std::ops::Deref;

use crate::actor::context::ActorContext;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::RouterActor;
use crate::routing::router_config::{Group, TRouterConfig};
use crate::system::ActorSystem;

#[derive(Clone)]
pub(crate) struct GroupRouterConfig {
    config: Box<dyn Group>,
}

impl Deref for GroupRouterConfig {
    type Target = Box<dyn Group>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl TRouterConfig for GroupRouterConfig {
    fn create_router(&self, system: ActorSystem) -> Router {
        self.config.create_router(system)
    }

    fn create_router_actor(&self) -> RouterActor {
        self.config.create_router_actor()
    }
}

impl Group for GroupRouterConfig {
    fn paths(&self, system: &ActorSystem) -> Vec<String> {
        self.config.paths(system)
    }

    fn props(&self) -> Props {
        self.config.props()
    }

    fn routee_for(&self, path: &String, context: &mut ActorContext) -> Box<dyn Routee> {
        self.config.routee_for(path, context)
    }
}