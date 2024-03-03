use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::props::{Props, PropsBuilder};
use crate::routing::routee::{ActorRefRoutee, Routee};
use crate::routing::router_config::pool::Pool;
use crate::routing::router_config::TRouterConfig;
use crate::routing::routing_logic::round_robin_routing_logic::RoundRobinRoutingLogic;
use crate::routing::routing_logic::RoutingLogic;

pub struct RoundRobinPool<A> where A: Clone + Send {
    pub nr_of_instances: usize,
    pub routee_props: PropsBuilder<A>,
    pub arg: A,
}

impl<A> RoundRobinPool<A> where A: Clone + Send {
    pub fn new(n: usize, routee_props: PropsBuilder<A>, arg: A) -> Self {
        Self {
            nr_of_instances: n,
            routee_props,
            arg,
        }
    }
}

impl<A> TRouterConfig for RoundRobinPool<A> where A: Clone + Send {
    fn routing_logic(&self) -> Box<dyn RoutingLogic> {
        Box::new(RoundRobinRoutingLogic::default())
    }

    fn props(&self) -> Props {
        todo!()
        // let router_config = PoolRouterConfig::new(self).into();
        // Props::new(move || {
        //     Ok(RouterActor::new(router_config))
        // })
    }
}

impl<A> Pool for RoundRobinPool<A> where A: Clone + Send {
    fn nr_of_instances(&self, _sys: &ActorSystem) -> usize {
        self.nr_of_instances
    }

    fn new_routee(&self, context: &mut ActorContext) -> anyhow::Result<Box<dyn Routee>> {
        let routee_props = self.routee_props.props(self.arg.clone());
        let routee = context.spawn_anonymous(routee_props)?;
        Ok(Box::new(ActorRefRoutee(routee)))
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use async_trait::async_trait;
    use tracing::{info, Level};

    use actor_derive::EmptyCodec;

    use crate::{Actor, Message};
    use crate::actor::actor_ref::ActorRefExt;
    use crate::actor::actor_ref_factory::ActorRefFactory;
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::props::{Props, PropsBuilder};
    use crate::config::actor_setting::ActorSetting;
    use crate::ext::{init_logger, type_name_of};
    use crate::routing::round_robin::RoundRobinPool;
    use crate::routing::router_config::TRouterConfig;

    #[derive(Debug)]
    struct TestActor;

    impl Actor for TestActor {}

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
        init_logger(Level::TRACE);
        let system = ActorSystem::create("mikai233", ActorSetting::default())?;
        let router_props = RoundRobinPool::new(
            5,
            PropsBuilder::new(type_name_of::<TestActor>(), |()| { Props::new(|| { Ok(TestActor) }) }),
            (),
        ).props();
        let round_robin_router = system.spawn_anonymous(router_props)?;
        for _ in 0..10 {
            round_robin_router.cast_ns(TestMessage);
        }
        // let another_routee = system.spawn_anonymous(Props::new_with_ctx(|_| Ok(TestActor)))?;
        // round_robin_router.cast_ns(AddRoutee { routee: Arc::new(Box::new(ActorRefRoutee(another_routee))) });
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}