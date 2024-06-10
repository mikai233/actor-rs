use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::props::{Props, PropsBuilder};
use crate::routing::routee::Routee;
use crate::routing::router_actor::{Router, RouterActor};
use crate::routing::router_config::{RouterConfig, RouterProps, TRouterConfig};
use crate::routing::router_config::pool::{Pool, PoolRouterConfig};
use crate::routing::routing_logic::round_robin_routing_logic::RoundRobinRoutingLogic;
use crate::routing::routing_logic::RoutingLogic;
use crate::routing::spawn_actor_routee;

pub struct RoundRobinPool<A> where A: Clone + Send + 'static {
    routing_logic: RoundRobinRoutingLogic,
    pub nr_of_instances: usize,
    pub routee_props: PropsBuilder<A>,
    pub arg: A,
}

impl<A> RoundRobinPool<A> where A: Clone + Send + 'static {
    pub fn new(n: usize, routee_props: PropsBuilder<A>, arg: A) -> Self {
        Self {
            routing_logic: RoundRobinRoutingLogic::default(),
            nr_of_instances: n,
            routee_props,
            arg,
        }
    }
}

impl<A> TRouterConfig for RoundRobinPool<A> where A: Clone + Send + 'static {
    fn routing_logic(&self) -> &dyn RoutingLogic {
        &self.routing_logic
    }
}

impl<A> Pool for RoundRobinPool<A> where A: Clone + Send + 'static {
    fn nr_of_instances(&self, _sys: &ActorSystem) -> usize {
        self.nr_of_instances
    }

    fn new_routee(&self, context: &mut ActorContext) -> anyhow::Result<Routee> {
        let routee = spawn_actor_routee(context, &self.routee_props, self.arg.clone())?;
        Ok(routee.into())
    }
}

impl<A> RouterProps for RoundRobinPool<A> where A: Clone + Send + 'static {
    fn props(self) -> Props {
        let router_config = RouterConfig::PoolRouterConfig(PoolRouterConfig::new(self));
        Props::new(move || {
            let router: Box<dyn Router> = Box::new(RouterActor::new(router_config));
            Ok(router)
        })
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use async_trait::async_trait;
    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, DynMessage, Message};
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::props::{Props, PropsBuilder};
    use crate::actor_ref::actor_ref_factory::ActorRefFactory;
    use crate::actor_ref::ActorRefExt;
    use crate::config::actor_setting::ActorSetting;
    use crate::routing::round_robin_pool::RoundRobinPool;
    use crate::routing::router_actor::routee_envelope::RouteeEnvelope;
    use crate::routing::router_config::RouterProps;

    #[derive(Debug)]
    struct TestActor;

    #[async_trait]
    impl Actor for TestActor {
        async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
            info!("{} started", context.myself());
            Ok(())
        }

        async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
            Self::handle_message(self, context, message).await
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct TestMessage;

    #[async_trait]
    impl Message for TestMessage {
        type A = TestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let myself = context.myself();
            info!("{} handle round robin message {:?}", myself, self);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_round_robin() -> anyhow::Result<()> {
        let system = ActorSystem::new("mikai233", ActorSetting::default())?;
        let router_props = RoundRobinPool::new(
            5,
            PropsBuilder::new(|()| { Ok(TestActor) }),
            (),
        ).props();
        let round_robin_router = system.spawn_anonymous(router_props)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        for _ in 0..200 {
            round_robin_router.cast_ns(RouteeEnvelope::new(TestMessage));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}