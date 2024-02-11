use std::ops::Deref;

use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::TRouterActor;
use crate::routing::router_config::{Pool, TRouterConfig};

#[derive(Clone)]
pub struct PoolRouterConfig {
    config: Box<dyn Pool>,
}

impl PoolRouterConfig {
    pub fn new<C>(config: C) -> Self where C: Pool {
        Self {
            config: Box::new(config)
        }
    }
}

impl Deref for PoolRouterConfig {
    type Target = Box<dyn Pool>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl TRouterConfig for PoolRouterConfig {
    fn create_router(&self) -> Router {
        self.config.create_router()
    }

    fn create_router_actor(&self, routee_props: Props) -> anyhow::Result<Box<dyn TRouterActor>> {
        self.config.create_router_actor(routee_props)
    }
}

impl Pool for PoolRouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize {
        self.config.nr_of_instances(sys)
    }

    fn new_routee(&self, routee_props: Props, context: &mut ActorContext) -> anyhow::Result<Box<dyn Routee>> {
        self.config.new_routee(routee_props, context)
    }

    fn props(&self, routee_props: Props) -> Props {
        self.config.props(routee_props)
    }

    fn supervisor_strategy(&self) -> &Box<dyn SupervisorStrategy> {
        self.config.supervisor_strategy()
    }
}