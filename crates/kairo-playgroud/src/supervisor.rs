use anyhow::anyhow;
use async_trait::async_trait;
use tracing::{Level, info};

use kairo_core::EmptyCodec;
use kairo_core::actor::actor_system::ActorSystem;
use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::actor::props::Props;
use kairo_core::actor_ref::ActorRefExt;
use kairo_core::actor_ref::actor_ref_factory::ActorRefFactory;
use kairo_core::config::actor_setting::ActorSetting;
use kairo_core::ext::init_logger;
use kairo_core::{Actor, DynMessage, Message};

struct TestActor;

#[async_trait]
impl Actor for TestActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(
        &mut self,
        context: &mut ActorContext,
        message: DynMessage,
    ) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[derive(EmptyCodec)]
struct ErrorMessage;

#[async_trait]
impl Message for ErrorMessage {
    type A = TestActor;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        Err(anyhow!("test error message"))
    }
}

#[derive(Debug, EmptyCodec)]
struct NormalMessage;

#[async_trait]
impl Message for NormalMessage {
    type A = TestActor;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
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
