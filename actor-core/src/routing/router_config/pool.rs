use std::ops::Deref;

use crate::actor::actor_system::ActorSystem;
use crate::actor::context::Context;
use crate::routing::routee::Routee;
use crate::routing::router_config::TRouterConfig;
use crate::routing::routing_logic::RoutingLogic;

pub trait Pool: TRouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize;

    fn new_routee(&self, context: &mut Context) -> anyhow::Result<Routee>;
}

pub struct PoolRouterConfig {
    pool: Box<dyn Pool>,
}

impl PoolRouterConfig {
    pub fn new<P>(pool: P) -> Self where P: Pool + 'static {
        Self {
            pool: Box::new(pool)
        }
    }
}

impl Deref for PoolRouterConfig {
    type Target = Box<dyn Pool>;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl TRouterConfig for PoolRouterConfig {
    fn routing_logic(&self) -> &dyn RoutingLogic {
        self.pool.routing_logic()
    }
}

impl Pool for PoolRouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize {
        self.pool.nr_of_instances(sys)
    }

    fn new_routee(&self, context: &mut Context) -> anyhow::Result<Routee> {
        self.pool.new_routee(context)
    }
}