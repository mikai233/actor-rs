use crate::actor::actor_system::ActorSystem;
use crate::actor::context::Context;
use crate::actor::props::{Props, PropsBuilder};
use crate::routing::routee::Routee;
use crate::routing::router_actor::RouterActor;
use crate::routing::router_config::pool::{Pool, PoolRouterConfig};
use crate::routing::router_config::{RouterConfig, RouterProps, TRouterConfig};
use crate::routing::routing_logic::round_robin_routing_logic::RoundRobinRoutingLogic;
use crate::routing::routing_logic::RoutingLogic;
use crate::routing::spawn_actor_routee;

pub struct RoundRobinPool<A>
where
    A: Clone + Send + 'static,
{
    routing_logic: RoundRobinRoutingLogic,
    pub nr_of_instances: usize,
    pub routee_props: PropsBuilder<A>,
    pub arg: A,
}

impl<A> RoundRobinPool<A>
where
    A: Clone + Send + 'static,
{
    pub fn new(n: usize, routee_props: PropsBuilder<A>, arg: A) -> Self {
        Self {
            routing_logic: RoundRobinRoutingLogic::default(),
            nr_of_instances: n,
            routee_props,
            arg,
        }
    }
}

impl<A> TRouterConfig for RoundRobinPool<A>
where
    A: Clone + Send + 'static,
{
    fn routing_logic(&self) -> &dyn RoutingLogic {
        &self.routing_logic
    }
}

impl<A> Pool for RoundRobinPool<A>
where
    A: Clone + Send + 'static,
{
    fn nr_of_instances(&self, _sys: &ActorSystem) -> usize {
        self.nr_of_instances
    }

    fn new_routee(&self, context: &mut Context) -> anyhow::Result<Routee> {
        let routee = spawn_actor_routee(context, &self.routee_props, self.arg.clone())?;
        Ok(routee.into())
    }
}

impl<A> RouterProps for RoundRobinPool<A>
where
    A: Clone + Send + 'static,
{
    fn props(self) -> Props {
        let router_config = RouterConfig::PoolRouterConfig(PoolRouterConfig::new(self));
        Props::new(move || {
            Ok(RouterActor::new(router_config))
        })
    }
}

#[cfg(test)]
mod test {
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::receive::Receive;
    use crate::actor::Actor;
    use actor_derive::Message;

    #[derive(Debug)]
    struct TestActor;

    impl Actor for TestActor {
        type Context = Context;

        fn receive(&self) -> Receive<Self> {
            Receive::new()
        }
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("TestMessage")]
    struct TestMessage;

    // #[tokio::test]
    // async fn test_round_robin() -> anyhow::Result<()> {
    //     let system = ActorSystem::new("mikai233", ActorSetting::default())?;
    //     let router_props =
    //         RoundRobinPool::new(5, PropsBuilder::new(|()| Ok(TestActor)), ()).props();
    //     let round_robin_router = system.spawn_anonymous(router_props)?;
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     for _ in 0..200 {
    //         round_robin_router.cast_ns(RouteeEnvelope::new(TestMessage));
    //     }
    //     tokio::time::sleep(Duration::from_secs(2)).await;
    //     Ok(())
    // }
}
