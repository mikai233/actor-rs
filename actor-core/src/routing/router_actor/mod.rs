use crate::actor::context::{ActorContext, Context};
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::routing::routee::Routee;
use crate::routing::router_config::pool::Pool;
use crate::routing::router_config::RouterConfig;

pub mod routee_envelope;
mod routee_terminated;
mod get_routees;
mod add_routee;
mod remove_routee;
pub mod broadcast;

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

impl Actor for RouterActor {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let context = ctx.context_mut();
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

    fn receive(&self) -> Receive<Self> {
        todo!()
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