use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use actor_core::{Actor, Message};
use actor_core::context::ActorContext;
use actor_core::ext::init_logger;
use actor_core::props::Props;
use actor_core::provider::ActorRefFactory;
use actor_core::system::ActorSystem;
use actor_core::system::config::Config;
use actor_derive::{EmptyCodec, MessageCodec};

#[derive(EmptyCodec)]
struct LocalMessage;

impl Message for LocalMessage {
    type A = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle LocalMessage");
        Ok(())
    }
}

#[derive(Serialize, Deserialize, MessageCodec)]
#[actor(ActorA)]
struct RemoteMessage;

impl Message for RemoteMessage {
    type A = ActorA;

    fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("handle RemoteMessage");
        Ok(())
    }
}

struct ActorA;

impl Actor for ActorA {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system = ActorSystem::create(Config::default()).await?;
    system.spawn_anonymous_actor(Props::create(|_| ActorA))?;
    system.scheduler().start_timer_with_fixed_delay_with(
        None,
        Duration::from_secs(1),
        || info!("hello world"),
    );
    system.wait_termination().await;
    Ok(())
}
