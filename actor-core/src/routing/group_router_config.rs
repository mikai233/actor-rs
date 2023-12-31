use std::ops::Deref;

use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::TRouterActor;
use crate::routing::router_config::{Group, TRouterConfig};

#[derive(Clone)]
pub struct GroupRouterConfig {
    config: Box<dyn Group>,
}

impl Deref for GroupRouterConfig {
    type Target = Box<dyn Group>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl TRouterConfig for GroupRouterConfig {
    fn create_router(&self) -> Router {
        self.config.create_router()
    }

    fn create_router_actor(&self, routee_props: Props) -> Box<dyn TRouterActor> {
        self.config.create_router_actor(routee_props)
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