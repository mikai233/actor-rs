use std::sync::Arc;

use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::{AsAny, MessageCodec};

use crate::{Actor, DynMessage, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::ActorContext;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::ext::as_any::AsAny;
use crate::message::terminated::WatchTerminated;
use crate::routing::router::Router;
use crate::routing::router_config::{RouterConfig, TRouterConfig};

pub trait TRouterActor: Actor + AsAny {
    fn router(&mut self) -> &mut Router;

    fn props(&self) -> &Props;
}

#[async_trait]
impl Actor for Box<dyn TRouterActor> {
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

#[derive(AsAny)]
pub struct RouterActor {
    pub router: Router,
    pub props: Props,
}

#[async_trait]
impl Actor for RouterActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let routee_props = self.props.with_router(None);
        match self.props.router_config.as_ref().unwrap() {
            RouterConfig::PoolRouterConfig(pool) => {
                let nr_of_routees = pool.nr_of_instances(context.system());
                let mut routees = vec![];
                for _ in 0..nr_of_routees {
                    let routee = pool.new_routee(routee_props.clone(), context)?;
                    routees.push(Arc::new(routee));
                }
                self.router().routees.extend(routees);
            }
            RouterConfig::GroupRouterConfig(_group) => {
                todo!()
            }
        }
        Ok(())
    }

    fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        if self.props().router_config().unwrap().is_management_message(&message) {
            Some(message)
        } else {
            let sender = context.sender.take();
            self.router().route(context.system(), message, sender);
            None
        }
    }
}

impl TRouterActor for RouterActor {
    fn router(&mut self) -> &mut Router {
        &mut self.router
    }

    fn props(&self) -> &Props {
        &self.props
    }
}

#[derive(Decode, Encode, MessageCodec)]
pub(crate) struct WatchRouteeTerminated(ActorRef);

#[async_trait]
impl Message for WatchRouteeTerminated {
    type A = RouterActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl WatchTerminated for WatchRouteeTerminated {
    fn watch_actor(&self) -> &ActorRef {
        &self.0
    }
}