use std::ops::Deref;

use crate::actor::context::ActorContext;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::RouterActor;
use crate::routing::router_config::{Pool, TRouterConfig};
use crate::system::ActorSystem;

#[derive(Clone)]
pub(crate) struct PoolRouterConfig {
    config: Box<dyn Pool>,
}

impl Deref for PoolRouterConfig {
    type Target = Box<dyn Pool>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl TRouterConfig for PoolRouterConfig {
    fn create_router(&self, system: ActorSystem) -> Router {
        self.config.create_router(system)
    }

    fn create_router_actor(&self) -> RouterActor {
        self.config.create_router_actor()
    }
}

impl Pool for PoolRouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize {
        self.config.nr_of_instances(sys)
    }

    fn new_routee(&self, routee_props: Props, context: &mut ActorContext) -> Box<dyn Routee> {
        self.config.new_routee(routee_props, context)
    }

    fn props(&self, routee_props: Props) -> Props {
        self.config.props(routee_props)
    }
}