use anyhow::anyhow;
use async_trait::async_trait;
use tracing::{info, Level};

use actor_core::{Actor, Message};
use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger;
use actor_derive::EmptyCodec;

struct TestActor;

#[async_trait]
impl Actor for TestActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        Ok(())
    }
}

#[derive(EmptyCodec)]
struct ErrorMessage;

#[async_trait]
impl Message for ErrorMessage {
    type A = TestActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        Err(anyhow!("test error message"))
    }
}

#[derive(Debug, EmptyCodec)]
struct NormalMessage;

#[async_trait]
impl Message for NormalMessage {
    type A = TestActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("recv {:?}", *self);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::create("mikai233", ActorSetting::default())?;
    let test_actor = system.spawn_anonymous(Props::new_with_ctx(|_| Ok(TestActor)))?;
    test_actor.cast_ns(ErrorMessage);
    test_actor.cast_ns(NormalMessage);
    system.await;
    Ok(())
}