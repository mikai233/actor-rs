use async_trait::async_trait;

use crate::{Actor, DynMessage};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::ActorContext;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::routing::routee::Routee;
use crate::routing::router_config::pool::Pool;
use crate::routing::router_config::RouterConfig;

pub mod routee_envelope;
mod routee_terminated;
mod get_routees;
mod add_routee;
mod remove_routee;
mod broadcast;

pub trait Router: Actor {
    fn router_config(&self) -> &RouterConfig;

    fn routees_mut(&mut self) -> &mut Vec<Routee>;

    fn routees(&self) -> &Vec<Routee>;
}

#[derive(Debug)]
pub struct RouterActor {
    routees: Vec<Routee>,
    router_config: RouterConfig,
}

impl RouterActor {
    pub fn new(router_config: RouterConfig) -> Self {
        Self {
            routees: Default::default(),
            router_config,
        }
    }
}

#[async_trait]
impl Actor for RouterActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        match &self.router_config {
            RouterConfig::PoolRouterConfig(pool) => {
                let n = pool.nr_of_instances(context.system());
                for _ in 0..n {
                    let routee = pool.new_routee(context)?;
                    self.routees.push(routee);
                }
            }
            RouterConfig::GroupRouterConfig(_) => {}
        }
        Ok(())
    }
}

impl Router for RouterActor {
    fn router_config(&self) -> &RouterConfig {
        &self.router_config
    }

    fn routees_mut(&mut self) -> &mut Vec<Routee> {
        &mut self.routees
    }

    fn routees(&self) -> &Vec<Routee> {
        &self.routees
    }
}

#[async_trait]
impl Actor for Box<dyn Router> {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        (&mut **self).started(context).await
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        (&mut **self).stopped(context).await
    }

    fn supervisor_strategy(&self) -> Box<dyn SupervisorStrategy> {
        (&**self).supervisor_strategy()
    }

    fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        (&mut **self).on_recv(context, message)
    }
}