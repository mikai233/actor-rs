use std::net::SocketAddrV4;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{info, Level};

use actor_core::{EmptyTestActor, Message};
use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::deferred_ref::Patterns;
use actor_core::actor::props::Props;
use actor_core::ext::init_logger;
use actor_core::message::message_registration::MessageRegistration;
use actor_derive::{MessageCodec, OrphanCodec};
use actor_remote::remote_provider::RemoteActorRefProvider;

#[derive(Encode, Decode, MessageCodec)]
struct MessageToAsk;

#[async_trait]
impl Message for MessageToAsk {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.sender().unwrap().resp(MessageToAns {
            content: "hello world".to_string(),
        });
        Ok(())
    }
}

#[derive(Encode, Decode, OrphanCodec)]
struct MessageToAns {
    content: String,
}

fn build_config(addr: SocketAddrV4) -> ActorSystemConfig {
    let mut config = ActorSystemConfig::default();
    config.with_provider(move |system| {
        let mut registration = MessageRegistration::new();
        registration.register::<MessageToAsk>();
        registration.register::<MessageToAns>();
        RemoteActorRefProvider::new(system, registration, addr).map(|(r, d)| (r.into(), d))
    });
    config
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system1 = ActorSystem::create("mikai233", build_config("127.0.0.1:12121".parse()?))?;
    let system2 = ActorSystem::create("mikai233", build_config("127.0.0.1:12123".parse()?))?;
    let actor_a = system1.spawn_anonymous_actor(Props::create(|_| EmptyTestActor))?;
    let actor_a = system2.provider().resolve_actor_ref_of_path(actor_a.path());
    let start = SystemTime::now();
    for _ in 0..1000000 {
        let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await.unwrap();
    }
    let end = SystemTime::now();
    let cost = end.duration_since(start)?;
    info!("cost {:?}", cost);
    Ok(())
}
