use std::collections::HashSet;
use std::net::SocketAddrV4;

use clap::Parser;
use etcd_client::Client;

use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::cluster_setting::ClusterSetting;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::config::actor_system_config::ActorSystemConfig;
use actor_core::ext::init_logger_with_filter;
use actor_core::message::message_registration::MessageRegistration;

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
    let mut config = ActorSystemConfig::default();
    config.with_provider(move |system| {
        let reg = MessageRegistration::new();
        let setting = ClusterSetting::builder()
            .system(system.clone())
            .addr(args.addr)
            .reg(reg)
            .eclient(client.clone())
            .roles(HashSet::new())
            .build();
        ClusterActorRefProvider::new(setting).map(|(c, d)| (c.into(), d))
    });
    let system = ActorSystem::create(args.system_name, config)?;
    system.wait_termination().await;
    Ok(())
}