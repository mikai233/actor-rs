use std::io::Write;
use std::net::SocketAddrV4;
use std::time::{Duration, SystemTime};
use bincode::{Decode, Encode};
use pprof::protos::Message as PBMessage;
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
use actor_derive::{MessageCodec, UntypedMessageCodec};
use actor_remote::message_registration::MessageRegistration;
use actor_remote::remote_provider::RemoteActorRefProvider;

#[derive(Encode, Decode, MessageCodec)]
#[actor(EmptyTestActor)]
struct MessageToAsk;

impl Message for MessageToAsk {
    type A = EmptyTestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.sender().unwrap().resp(MessageToAns {
            content: "hello world".to_string(),
        });
        Ok(())
    }
}

#[derive(Encode, Decode, UntypedMessageCodec)]
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
    // let actor_a = system2.provider().resolve_actor_ref_of_path(actor_a.path());
    let guard = pprof::ProfilerGuard::new(10000).unwrap();
    let start = SystemTime::now();
    for _ in 0..1000000 {
        let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await.unwrap();
    }
    let end = SystemTime::now();
    let cost = end.duration_since(start)?;
    info!("cost {:?}", cost);
    match guard.report().build() {
        Ok(report) => {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
            let mut file = std::fs::File::create("profile.pb").unwrap();
            let profile = report.pprof().unwrap();

            let mut content = Vec::new();
            profile.write_to_vec(&mut content).unwrap();
            file.write_all(&content).unwrap();
        }
        Err(e) => {
            println!("{:?}", e);
        }
    };
    Ok(())
}
