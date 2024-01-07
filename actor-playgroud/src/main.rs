use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{info, Level};

use actor_core::{EmptyTestActor, Message};
use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::ext::init_logger;
use actor_core::message::message_registration::MessageRegistration;
use actor_derive::{CMessageCodec, MessageCodec, OrphanCodec};
use actor_remote::remote_provider::RemoteActorRefProvider;
use actor_remote::remote_setting::RemoteSetting;

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

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
struct TestMessage;

#[async_trait]
impl Message for TestMessage {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("{} recv {:?}", context.myself(), self);
        Ok(())
    }
}

fn build_config(addr: SocketAddrV4) -> ActorSystemConfig {
    let mut config = ActorSystemConfig::default();
    config.with_provider(move |system| {
        let mut reg = MessageRegistration::new();
        reg.register::<MessageToAsk>();
        reg.register::<MessageToAns>();
        let setting = RemoteSetting::builder()
            .system(system.clone())
            .addr(addr)
            .reg(reg)
            .build();
        RemoteActorRefProvider::new(setting).map(|(r, d)| (r.into(), d))
    });
    config
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger(Level::DEBUG);
    let system1 = ActorSystem::create("mikai233", build_config("127.0.0.1:12121".parse()?))?;
    let system2 = ActorSystem::create("mikai233", build_config("127.0.0.1:12123".parse()?))?;
    for i in 0..10 {
        system1.spawn_actor(Props::create(|_| EmptyTestActor), format!("test_actor_{}", i))?;
    }
    let sel = system1.actor_selection(ActorSelectionPath::RelativePath("/user/../user/test_actor_*".to_string()))?;
    let which = sel.resolve_one(Duration::from_secs(3)).await?;
    info!("{}", which);
    let sel = system2.actor_selection(ActorSelectionPath::FullPath("tcp://mikai233@127.0.0.1:12121/user/test_actor_9".parse()?))?;
    let which = sel.resolve_one(Duration::from_secs(3)).await?;
    info!("{}", which);
    // sel.tell(DynMessage::user(TestMessage), ActorRef::no_sender());
    system1.wait_termination().await;
    Ok(())
}
