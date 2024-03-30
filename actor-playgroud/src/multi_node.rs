use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::str::FromStr;

use clap::Parser;
use etcd_client::Client;
use futures::future::join_all;

use actor_cluster::cluster_provider::ClusterProviderBuilder;
use actor_cluster::config::ClusterConfig;
use actor_cluster_sharding::cluster_sharding::ClusterSharding;
use actor_cluster_sharding::config::ClusterShardingConfig;
use actor_cluster_sharding::register_sharding;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger_with_filter;
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "mikai233")]
    system_name: String,
    #[arg(short, long, default_value = "127.0.0.1:2379")]
    etcd: SocketAddrV4,
    #[arg(short, long, default_value = "3")]
    num: u16,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args { system_name, etcd, num } = Args::try_parse()?;
    init_logger_with_filter("debug,actor=debug,actor_core::actor::scheduler=info,actor_remote::remote_watcher=info,h2=info,tower=info,hyper=info");
    let client = Client::connect([etcd.to_string()], None).await?;
    let mut systems = vec![];
    for i in 0..num {
        let system_name = system_name.clone();
        let client = client.clone();
        let port = 2333 + i;
        let addr = SocketAddrV4::from_str(&format!("127.0.0.1:{}", port))?;
        let setting = ActorSetting::builder()
            .provider_fn(move |system| {
                let config = ClusterConfig {
                    remote: RemoteConfig { transport: Transport::tcp(addr, None) },
                    roles: HashSet::new(),
                };
                ClusterProviderBuilder::new()
                    .client(client.clone())
                    .register_all(register_sharding)
                    .config(config)
                    .build(system.clone())
            })
            .build();
        let system = ActorSystem::new(system_name, setting)?;
        system.register_extension(|system| {
            ClusterSharding::new(system, ClusterShardingConfig::default())
        })?;
        systems.push(system);
    }
    join_all(systems).await;
    Ok(())
}