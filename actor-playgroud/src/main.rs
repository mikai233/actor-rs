use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use etcd_client::Client;
use tracing::info;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::config::ClusterConfig;
use actor_core::{DynMessage, EmptyTestActor, Message};
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger_with_filter;
use actor_derive::{CMessageCodec, MessageCodec, OrphanCodec};
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

mod node;

#[derive(Encode, Decode, MessageCodec)]
struct MessageToAsk;

#[async_trait]
impl Message for MessageToAsk {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
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

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        info!("{} recv {:?}", context.myself(), self);
        Ok(())
    }
}

fn build_setting(addr: SocketAddrV4, client: Client) -> ActorSetting {
    ActorSetting::builder()
        .provider_fn(move |system| {
            let config = ClusterConfig {
                remote: RemoteConfig { transport: Transport::tcp(addr, None) },
                roles: Default::default(),
            };
            ClusterActorRefProvider::builder()
                .config(config)
                .client(client.clone())
                .register::<MessageToAsk>()
                .register::<MessageToAns>()
                .register::<TestMessage>()
                .build(system.clone())
        })
        .build()
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_logger_with_filter("actor=trace");
    let client = Client::connect(["localhost:2379"], None).await?;
    let system1 = ActorSystem::new("mikai233", build_setting("127.0.0.1:12121".parse()?, client.clone()))?;
    let system2 = ActorSystem::new("mikai233", build_setting("127.0.0.1:12123".parse()?, client.clone()))?;
    // tokio::spawn(async move {
    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     system2.terminate().await;
    // });
    for i in 0..10 {
        system1.spawn(Props::new_with_ctx(|_| Ok(EmptyTestActor)), format!("test_actor_{}", i))?;
    }
    let sel = system1.actor_selection(ActorSelectionPath::RelativePath("/user/../user/test_actor_*".to_string()))?;
    let which = sel.resolve_one(Duration::from_secs(3)).await?;
    info!("{}", which);
    // let sel = system2.actor_selection(ActorSelectionPath::FullPath("tcp://mikai233@127.0.0.1:12121/user/test_actor_9".parse()?))?;
    // let which = sel.resolve_one(Duration::from_secs(a3)).await?;
    // info!("{}", which);
    sel.tell(DynMessage::user(TestMessage), ActorRef::no_sender());
    {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let cluster = Cluster::get(&system2);
        let m = cluster.members();
        info!("aaaaaaaaa{:?}", m);
        cluster.leave(Address::new("tcp", "mikai233", Some("127.0.0.1:12123".parse()?)));
    }
    system2.await?;
    Ok(())
}
