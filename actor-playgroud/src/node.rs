use std::net::SocketAddrV4;

use clap::Parser;
use etcd_client::Client;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::ext::init_logger_with_filter;
use actor_playgroud::common::build_cluster_setting;

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
    let Args { system_name, addr, etcd } = Args::parse();
    init_logger_with_filter("actor=trace");
    let client = Client::connect([etcd.to_string()], None).await?;
    let setting = build_cluster_setting(addr, client)?;
    let system = ActorSystem::new(system_name, setting)?;
    system.await?;
    Ok(())
}