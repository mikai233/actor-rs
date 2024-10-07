use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext1;
use crate::actor::props::{Props, PropsBuilder};
use crate::routing::routee::Routee;
use crate::routing::router_actor::{Router, RouterActor};
use crate::routing::router_config::{RouterConfig, RouterProps, TRouterConfig};
use crate::routing::router_config::pool::{Pool, PoolRouterConfig};
use crate::routing::routing_logic::broadcast_routing_logic::BroadcastRoutingLogic;
use crate::routing::routing_logic::RoutingLogic;
use crate::routing::spawn_actor_routee;

pub struct BroadcastPool<A> where A: Clone + Send + 'static {
    routing_logic: BroadcastRoutingLogic,
    pub nr_of_instances: usize,
    pub routee_props: PropsBuilder<A>,
    pub arg: A,
}

impl<A> BroadcastPool<A> where A: Clone + Send + 'static {
    pub fn new(n: usize, routee_props: PropsBuilder<A>, arg: A) -> Self {
        Self {
            routing_logic: BroadcastRoutingLogic::default(),
            nr_of_instances: n,
            routee_props,
            arg,
        }
    }
}

impl<A> TRouterConfig for BroadcastPool<A> where A: Clone + Send + 'static {
    fn routing_logic(&self) -> &dyn RoutingLogic {
        &self.routing_logic
    }
}

impl<A> Pool for BroadcastPool<A> where A: Clone + Send + 'static {
    fn nr_of_instances(&self, _sys: &ActorSystem) -> usize {
        self.nr_of_instances
    }

    fn new_routee(&self, context: &mut ActorContext1) -> anyhow::Result<Routee> {
        let routee = spawn_actor_routee(context, &self.routee_props, self.arg.clone())?;
        Ok(routee.into())
    }
}

impl<A> RouterProps for BroadcastPool<A> where A: Clone + Send + 'static {
    fn props(self) -> Props {
        let router_config = RouterConfig::PoolRouterConfig(PoolRouterConfig::new(self));
        Props::new(move || {
            let router: Box<dyn Router> = Box::new(RouterActor::new(router_config));
            Ok(router)
        })
    }
}