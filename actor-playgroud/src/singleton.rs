use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use clap::Parser;
use etcd_client::Client;
use tracing::info;

use actor_cluster::cluster_provider::ClusterProviderBuilder;
use actor_cluster::config::ClusterConfig;
use actor_cluster_tools::singleton::cluster_singleton_manager::{ClusterSingletonManager, ClusterSingletonManagerSettings};
use actor_cluster_tools::singleton::cluster_singleton_proxy::cluster_singleton_proxy_settings::ClusterSingletonProxySettings;
use actor_cluster_tools::singleton::cluster_singleton_proxy::ClusterSingletonProxy;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::{init_logger_with_filter, type_name_of};
use actor_derive::{CEmptyCodec, MessageCodec};
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "mikai233")]
    system_name: String,
    #[arg(short, long)]
    addr: SocketAddrV4,
    #[arg(short, long, default_value = "127.0.0.1:2379")]
    etcd: SocketAddrV4,
    #[arg(short, long)]
    name: String,
    #[arg(short, long, default_value = "false")]
    proxy: bool,
}

#[derive(Debug)]
struct SingletonActor;

#[async_trait]
impl Actor for SingletonActor {}

#[derive(Debug, Encode, Decode, MessageCodec)]
struct Greet(usize);

#[async_trait]
impl Message for Greet {
    type A = SingletonActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("{:?}", *self);
        info!("{} recv {:?}", context.myself(), *self);
        Ok(())
    }
}

#[derive(Debug, Clone, CEmptyCodec)]
struct StopSingleton;

#[async_trait]
impl Message for StopSingleton {
    type A = SingletonActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("stop singleton {}", context.myself());
        context.stop(context.myself());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    init_logger_with_filter("actor=debug,actor-core::scheduler=info");
    let client = Client::connect([args.etcd.to_string()], None).await?;
    let setting = ActorSetting::builder()
        .provider_fn(move |system| {
            let config = ClusterConfig {
                remote: RemoteConfig { transport: Transport::tcp(args.addr, None) },
                roles: HashSet::new(),
            };
            ClusterProviderBuilder::new()
                .client(client.clone())
                .register::<Greet>()
                .config(config)
                .build(system.clone())
        })
        .build();
    let system = ActorSystem::new(args.system_name, setting)?;
    if args.proxy {
        let path = format!("/user/{}", args.name);
        let settings = ClusterSingletonProxySettings::builder()
            .buffer_size(1000)
            .singleton_identification_interval(Duration::from_secs(3))
            .build();
        let proxy_props = ClusterSingletonProxy::props(path, settings);
        let proxy = system.spawn_anonymous(proxy_props)?;
        tokio::spawn(async move {
            let mut index = 0;
            loop {
                proxy.cast_ns(Greet(index));
                index += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    } else {
        let settings = ClusterSingletonManagerSettings::builder()
            .build();
        let singleton_props = ClusterSingletonManager::props(
            PropsBuilder::new::<SingletonActor, _>(|()| {
                Props::new(|| { Ok(SingletonActor) })
            }),
            DynMessage::user(StopSingleton),
            settings,
        )?;
        system.spawn(singleton_props, args.name)?;
    }
    system.await?;
    Ok(())
}