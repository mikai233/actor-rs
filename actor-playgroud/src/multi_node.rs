use std::net::SocketAddrV4;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use etcd_client::Client;
use futures::future::join_all;
use rand::random;

use actor_cluster_sharding::cluster_sharding::ClusterSharding;
use actor_cluster_sharding::cluster_sharding_settings::ClusterShardingSettings;
use actor_cluster_sharding::config::ClusterShardingConfig;
use actor_cluster_sharding::shard_allocation_strategy::least_shard_allocation_strategy::LeastShardAllocationStrategy;
use actor_cluster_sharding::ShardEnvelope;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor_ref::ActorRefExt;
use actor_core::config::ConfigBuilder;
use actor_core::ext::init_logger_with_filter;
use actor_core::ext::message_ext::UserMessageExt;
use actor_playgroud::common::actor_sharding_setting;
use actor_playgroud::common::handoff_player::HandoffPlayer;
use actor_playgroud::common::hello::Hello;
use actor_playgroud::common::init::Init;
use actor_playgroud::common::player_actor::player_actor_builder;
use actor_playgroud::common::player_message_extractor::PlayerMessageExtractor;

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
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let Args { system_name, etcd, num } = Args::try_parse()?;
    init_logger_with_filter("debug,actor=debug,actor_core::actor::scheduler=info,actor_remote::remote_watcher=info,h2=info,tower=info,hyper=info");
    let client = Client::connect([etcd.to_string()], None).await?;
    let mut systems = vec![];
    let mut players = vec![];
    for _ in 0..3 {
        players.push(random::<u64>());
    }
    for i in 0..num {
        let system_name = system_name.clone();
        let client = client.clone();
        let port = 2333 + i;
        let addr = SocketAddrV4::from_str(&format!("127.0.0.1:{}", port))?;
        let setting = actor_sharding_setting(addr, client)?;
        let system = ActorSystem::new(system_name, setting)?;
        system.register_extension(|system| {
            let config = ClusterShardingConfig::builder().build()?;
            ClusterSharding::new(system, config)
        })?;
        let builder = player_actor_builder();
        let settings = ClusterShardingSettings::create(&system);
        let strategy = LeastShardAllocationStrategy::new(&system, 1, 1.0);
        let player_shard_region = ClusterSharding::get(&system).start("player", builder, settings.into(), PlayerMessageExtractor, strategy, HandoffPlayer.into_dyn()).await?;
        let players = players.clone();
        system.handle().spawn(async move {
            let mut index = 0;
            for player_id in &players {
                player_shard_region.cast_ns(ShardEnvelope::new(player_id.to_string(), Init));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            loop {
                for player_id in &players {
                    let hello = Hello { index, data: vec![] };
                    player_shard_region.cast_ns(ShardEnvelope::new(player_id.to_string(), hello));
                    index += 1;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        });
        systems.push(system);
    }
    join_all(systems).await;
    Ok(())
}