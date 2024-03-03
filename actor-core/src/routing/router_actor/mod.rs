use async_trait::async_trait;

use crate::{Actor, DynMessage};
use crate::actor::context::ActorContext;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::routing::routee::Routee;
use crate::routing::router_config::RouterConfig;

mod routee_envelope;
mod routee_terminated;
mod get_routees;
mod add_routee;
mod remove_routee;
mod broadcast;
mod get_routees_resp;

pub trait Router: Actor {
    fn router_config(&self) -> &RouterConfig;

    fn routees(&mut self) -> &mut Vec<Box<dyn Routee>>;
}

#[derive(Debug)]
pub struct RouterActor {
    routees: Vec<Box<dyn Routee>>,
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
        // let routee_props = self.props.with_router(None);
        // match self.props.router_config.as_ref().unwrap() {
        //     RouterConfig::PoolRouterConfig(pool) => {
        //         let nr_of_routees = pool.nr_of_instances(context.system());
        //         let mut routees = vec![];
        //         for _ in 0..nr_of_routees {
        //             let routee = pool.new_routee(routee_props.clone(), context)?;
        //             routees.push(Arc::new(routee));
        //         }
        //         self.router().routees.extend(routees);
        //     }
        //     RouterConfig::GroupRouterConfig(_group) => {
        //         todo!()
        //     }
        // }
        Ok(())
    }
}

impl Router for RouterActor {
    fn router_config(&self) -> &RouterConfig {
        &self.router_config
    }

    fn routees(&mut self) -> &mut Vec<Box<dyn Routee>> {
        &mut self.routees
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