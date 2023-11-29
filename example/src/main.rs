use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use actor::{Actor, Message};
use actor::context::ActorContext;
use actor::ext::init_logger;
use actor::props::Props;
use actor::provider::ActorRefFactory;
use actor::system::ActorSystem;
use actor::system::config::Config;
use actor_derive::{EmptyCodec, MessageCodec};

#[derive(EmptyCodec)]
struct LocalMessage;

impl Message for LocalMessage {
    type A = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle LocalMessage");
        Ok(())
    }
}

#[derive(Serialize, Deserialize, MessageCodec)]
#[actor(ActorA)]
struct RemoteMessage;

impl Message for RemoteMessage {
    type A = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle RemoteMessage");
        Ok(())
    }
}

struct ActorA;

#[async_trait]
impl Actor for ActorA {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::create(Config::default()).await?;
    system.spawn_actor(Props::create(|_| ActorA), None)?;
    system.scheduler().start_timer_with_fixed_delay_with(
        None,
        Duration::from_secs(1),
        || Box::new(|| info!("hello world")),
    );
    system.wait_termination().await;
    Ok(())
}
