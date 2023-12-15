use std::time::Duration;
use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::fault_handing::{AllForOneStrategy, Directive, SupervisorStrategy};
use actor_core::actor::props::Props;
use actor_core::ext::init_logger;
use actor_core::ext::option_ext::OptionExt;
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

#[derive(EmptyCodec)]
struct LocalMessageFix {
    message: Box<LocalMessage>,
}

impl Message for LocalMessageFix {
    type A = TestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("fix logic");
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
        context.children().first().foreach(|child| {
            child.cast(TestChildError, ActorRef::no_sender());
        });
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct TestPanic;

impl Message for TestPanic {
    type A = TestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug)]
struct TestActor;

#[async_trait]
impl Actor for TestActor {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{:?} pre start", self);
        for _ in 0..3 {
            context.spawn_anonymous_actor(Props::create(|_| ChildActor))?;
        }
        Ok(())
    }

    fn supervisor_strategy(&self) -> Box<dyn SupervisorStrategy> {
        let s = AllForOneStrategy {
            max_nr_of_retries: 3,
            within_time_range: Some(Duration::from_secs(3)),
            directive: Directive::Restart,
        };
        Box::new(s)
    }

    fn handle_message(&mut self, _context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        if message.name == std::any::type_name::<LocalMessage>() {
            match message.downcast_into_raw::<TestActor, LocalMessage>() {
                Ok(message) => {
                    let fix = LocalMessageFix { message };
                    Some(DynMessage::user(fix))
                }
                Err(_) => {
                    None
                }
            }
        } else {
            Some(message)
        }
    }
}

#[derive(Debug)]
struct ChildActor;

#[async_trait]
impl Actor for ChildActor {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} pre start", context.myself());
        Ok(())
    }

    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} post stop", context.myself());
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct TestChildError;

impl Message for TestChildError {
    type A = ChildActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("{} handle {:?}", context.myself(), self);
        Err(anyhow!("test child error"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::create("mikai233", ActorSystemConfig::default())?;
    let test_actor = system.spawn_anonymous_actor(Props::create(|_| TestActor))?;
    test_actor.cast(LocalMessage, ActorRef::no_sender());
    // for _ in 0..10 {
    //     test_actor.cast(TestError, ActorRef::no_sender());
    //     tokio::time::sleep(Duration::from_secs(2)).await;
    // }
    test_actor.cast(TestPanic, ActorRef::no_sender());
    tokio::time::sleep(Duration::from_secs(3)).await;
    test_actor.cast(TestPanic, ActorRef::no_sender());
    system.wait_termination().await;
    Ok(())
}
