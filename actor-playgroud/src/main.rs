use std::time::Duration;
use anyhow::anyhow;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use actor_core::{Actor, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
use actor_core::actor::context::ActorContext;
use actor_core::actor::props::Props;
use actor_core::ext::init_logger;
use actor_core::message::recreate::Recreate;
use actor_derive::{EmptyCodec, MessageCodec};

#[derive(EmptyCodec)]
struct LocalMessage;

impl Message for LocalMessage {
    type A = TestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle LocalMessage");
        context.execute::<_, TestActor>(|_, actor| {
            info!("world hello");
            Ok(())
        });
        Ok(())
    }
}

#[derive(Serialize, Deserialize, MessageCodec)]
#[actor(TestActor)]
struct RemoteMessage;

impl Message for RemoteMessage {
    type A = TestActor;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle RemoteMessage");
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct TestError;

impl Message for TestError {
    type A = TestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        Err(anyhow!("test error"))
    }
}

#[derive(Debug)]
struct TestActor;

#[async_trait]
impl Actor for TestActor {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{:?} pre start", self);
        context.spawn_anonymous_actor(Props::create(|_| ChildActor))?;
        Ok(())
    }
}

#[derive(Debug)]
struct ChildActor;

#[async_trait]
impl Actor for ChildActor {
    async fn pre_start(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{:?} pre start", self);
        Ok(())
    }

    async fn post_stop(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{:?} post stop", self);
        Ok(())
    }
}

#[derive(Debug)]
struct Cluster;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::create("mikai233", ActorSystemConfig::default())?;
    system.register_extension(|system| async {
        Cluster
    }).await;
    let cluster = system.get_extension::<Cluster>().unwrap();
    info!("{:?}", cluster);
    let test_actor = system.spawn_anonymous_actor(Props::create(|_| TestActor))?;
    test_actor.cast(LocalMessage, ActorRef::no_sender());
    system.scheduler().start_timer_with_fixed_delay_with(
        None,
        Duration::from_secs(1),
        || info!("hello world"),
    );
    test_actor.cast_system(Recreate { error: None }, ActorRef::no_sender());
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        test_actor.cast(TestError, ActorRef::no_sender());
    });
    system.wait_termination().await;
    Ok(())
}
