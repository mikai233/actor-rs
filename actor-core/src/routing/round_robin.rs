use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::actor::actor_system::ActorSystem;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::DynMessage;
use crate::routing::pool_router_config::PoolRouterConfig;
use crate::routing::router::{MaybeRef, NoRoutee, Routee, Router, RoutingLogic};
use crate::routing::router_actor::{RouterActor, TRouterActor};
use crate::routing::router_config::{Pool, TRouterConfig};

#[derive(Debug, Default)]
pub struct RoundRobinRoutingLogic {
    next: AtomicUsize,
}

impl Clone for RoundRobinRoutingLogic {
    fn clone(&self) -> Self {
        Self {
            next: AtomicUsize::default(),
        }
    }
}

impl RoutingLogic for RoundRobinRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Arc<Box<dyn Routee>>>) -> MaybeRef<'a, Box<dyn Routee>> {
        if !routees.is_empty() {
            let size = routees.len();
            let index = self.next.fetch_add(1, Ordering::Relaxed) % size;
            MaybeRef::Ref(&routees[index])
        } else {
            MaybeRef::Own(Box::new(NoRoutee))
        }
    }
}

#[derive(Clone)]
pub struct RoundRobinPool {
    pub nr_of_instances: usize,
    pub supervisor_strategy: Box<dyn SupervisorStrategy>,
}

impl RoundRobinPool {
    pub fn new<S>(n: usize, strategy: S) -> Self where S: SupervisorStrategy {
        Self {
            nr_of_instances: n,
            supervisor_strategy: Box::new(strategy),
        }
    }
}

impl TRouterConfig for RoundRobinPool {
    fn create_router(&self) -> Router {
        Router::new(RoundRobinRoutingLogic::default())
    }

    fn create_router_actor(&self, routee_props: Props) -> Box<dyn TRouterActor> {
        let router = self.create_router();
        let router_actor = RouterActor {
            router,
            props: routee_props,
        };
        Box::new(router_actor)
    }
}

impl Pool for RoundRobinPool {
    fn nr_of_instances(&self, _sys: &ActorSystem) -> usize {
        self.nr_of_instances
    }

    fn props(&self, routee_props: Props) -> Props {
        let config = PoolRouterConfig::new(self.clone());
        routee_props.with_router(Some(config.into()))
    }

    fn supervisor_strategy(&self) -> &Box<dyn SupervisorStrategy> {
        &self.supervisor_strategy
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::Duration;

    use tracing::{info, Level};

    use actor_derive::EmptyCodec;

    use crate::{Actor, Message};
    use crate::actor::actor_ref::ActorRefExt;
    use crate::actor::actor_ref_factory::ActorRefFactory;
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::config::actor_system_config::ActorSystemConfig;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::fault_handing::OneForOneStrategy;
    use crate::actor::props::Props;
    use crate::ext::init_logger;
    use crate::routing::round_robin::RoundRobinPool;
    use crate::routing::router::ActorRefRoutee;
    use crate::routing::router_config::{AddRoutee, Pool};

    #[derive(Debug)]
    struct TestActor;

    impl Actor for TestActor {}

    #[derive(Debug, EmptyCodec)]
    struct TestMessage;

    impl Message for TestMessage {
        type A = TestActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let myself = context.myself();
            info!("{} handle round robin message {:?}", myself, self);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_round_robin() -> anyhow::Result<()> {
        init_logger(Level::TRACE);
        let system = ActorSystem::create("mikai233", ActorSystemConfig::default())?;
        let router_props = RoundRobinPool::new(5, OneForOneStrategy::default()).props(Props::create(|_| TestActor));
        let round_robin_router = system.spawn_anonymous_actor(router_props)?;
        for _ in 0..10 {
            round_robin_router.cast_ns(TestMessage);
        }
        let another_routee = system.spawn_anonymous_actor(Props::create(|_| TestActor))?;
        round_robin_router.cast_ns(AddRoutee { routee: Arc::new(Box::new(ActorRefRoutee(another_routee))) });
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}