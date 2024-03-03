use std::ops::Deref;

use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::props::Props;
use crate::routing::routee::Routee;
use crate::routing::router_config::TRouterConfig;
use crate::routing::routing_logic::RoutingLogic;

pub trait Group: TRouterConfig {
    fn paths(&self, system: &ActorSystem) -> Vec<String>;

    fn routee_for(&self, path: &String, context: &mut ActorContext) -> Box<dyn Routee>;
}

pub struct GroupRouterConfig {
    group: Box<dyn Group>,
}

impl GroupRouterConfig {
    pub fn new<G>(group: G) -> Self where G: Group + 'static {
        Self {
            group: Box::new(group)
        }
    }
}

impl Deref for GroupRouterConfig {
    type Target = Box<dyn Group>;

    fn deref(&self) -> &Self::Target {
        &self.group
    }
}

impl TRouterConfig for GroupRouterConfig {
    fn routing_logic(&self) -> Box<dyn RoutingLogic> {
        self.group.routing_logic()
    }

    fn stop_router_when_all_routees_removed(&self) -> bool {
        self.group.stop_router_when_all_routees_removed()
    }

    fn props(&self) -> Props {
        self.group.props()
    }
}

impl Group for GroupRouterConfig {
    fn paths(&self, system: &ActorSystem) -> Vec<String> {
        self.group.paths(system)
    }

    fn routee_for(&self, path: &String, context: &mut ActorContext) -> Box<dyn Routee> {
        self.group.routee_for(path, context)
    }
}