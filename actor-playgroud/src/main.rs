use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use actor_core::{Actor, Message};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::ActorContext;
use actor_core::ext::init_logger;
use actor_core::actor::props::Props;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
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
    system.spawn_anonymous_actor(Props::create(|_| ActorA))?;
    system.scheduler().start_timer_with_fixed_delay_with(
        None,
        Duration::from_secs(1),
        || info!("hello world"),
    );
    system.wait_termination().await;
    Ok(())
}
