use std::net::SocketAddrV4;
use std::time::Duration;

use clap::Parser;
use etcd_client::Client;

use actor_cluster_tools::singleton::cluster_singleton_manager::{ClusterSingletonManager, ClusterSingletonManagerSettings};
use actor_cluster_tools::singleton::cluster_singleton_proxy::cluster_singleton_proxy_settings::ClusterSingletonProxySettings;
use actor_cluster_tools::singleton::cluster_singleton_proxy::ClusterSingletonProxy;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::props::PropsBuilder;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::DynMessage;
use actor_core::ext::init_logger_with_filter;
use actor_playgroud::common::build_cluster_setting;
use actor_playgroud::common::greet::Greet;
use actor_playgroud::common::singleton_actor::SingletonActor;
use actor_playgroud::common::stop_singleton::StopSingleton;

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

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let Args { system_name, addr, etcd, name, proxy } = Args::parse();
    init_logger_with_filter("actor=debug,actor-core::scheduler=info");
    let client = Client::connect([etcd.to_string()], None).await?;
    let setting = build_cluster_setting(addr, client)?;
    let system = ActorSystem::new(system_name, setting)?;
    if proxy {
        let path = format!("/user/{}", name);
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
            PropsBuilder::new(|()| { Ok(SingletonActor) }),
            DynMessage::user(StopSingleton),
            settings,
        )?;
        system.spawn(singleton_props, name)?;
    }
    system.await?;
    Ok(())
}