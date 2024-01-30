use std::collections::HashSet;
use std::net::SocketAddrV4;

use clap::Parser;
use etcd_client::Client;

use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::cluster_setting::ClusterSetting;
use actor_cluster::config::ClusterConfig;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger_with_filter;
use actor_core::message::message_registration::MessageRegistration;
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::{TcpTransport, Transport};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "mikai233")]
    system_name: String,
    #[arg(short, long)]
    addr: SocketAddrV4,
    #[arg(short, long, default_value = "127.0.0.1:2379")]
    etcd: SocketAddrV4,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    init_logger_with_filter("actor=trace");
    let client = Client::connect([args.etcd.to_string()], None).await?;
    let mut setting = ActorSetting::default();
    setting.with_provider(move |system| {
        let reg = MessageRegistration::new();
        let config = ClusterConfig {
            remote: RemoteConfig { transport: Transport::Tcp(TcpTransport { addr: args.addr, buffer: None }) },
            roles: HashSet::new(),
        };
        let setting = ClusterSetting::builder()
            .system(system.clone())
            .config(config)
            .reg(reg)
            .eclient(client.clone())
            .build();
        ClusterActorRefProvider::new(setting).map(|(c, d)| (c.into(), d))
    });
    let system = ActorSystem::create(args.system_name, setting)?;
    system.await;
    Ok(())
}