use anyhow::anyhow;
use async_trait::async_trait;
use tracing::{info, Level};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::EmptyCodec;
use actor_core::ext::init_logger;

struct TestActor;

#[async_trait]
impl Actor for TestActor {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[derive(EmptyCodec)]
struct ErrorMessage;

#[async_trait]
impl Message for ErrorMessage {
    type A = TestActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext1, _actor: &mut Self::A) -> anyhow::Result<()> {
        Err(anyhow!("test error message"))
    }
}

#[derive(Debug, EmptyCodec)]
struct NormalMessage;

#[async_trait]
impl Message for NormalMessage {
    type A = TestActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext1, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("recv {:?}", *self);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::new("mikai233", ActorSetting::default())?;
    let test_actor = system.spawn_anonymous(Props::new(|| Ok(TestActor)))?;
    test_actor.cast_ns(ErrorMessage);
    test_actor.cast_ns(NormalMessage);
    system.await?;
    Ok(())
}