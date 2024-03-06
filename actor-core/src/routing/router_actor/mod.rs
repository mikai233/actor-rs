use anyhow::Error;
use async_trait::async_trait;

use crate::{Actor, DynMessage};
use crate::actor::context::ActorContext;
use crate::actor::directive::Directive;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::ActorRef;
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

    fn on_child_failure(&mut self, context: &mut ActorContext, child: &ActorRef, error: &Error) -> Directive {
        (&mut **self).on_child_failure(context, child, error)
    }

    fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        (&mut **self).on_recv(context, message)
    }
}