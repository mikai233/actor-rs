use std::net::SocketAddrV4;
use std::time::Duration;

use clap::Parser;
use etcd_client::Client;
use rand::random;

use actor_cluster_sharding::cluster_sharding::ClusterSharding;
use actor_cluster_sharding::cluster_sharding_settings::ClusterShardingSettings;
use actor_cluster_sharding::shard_allocation_strategy::least_shard_allocation_strategy::LeastShardAllocationStrategy;
use actor_cluster_sharding::ShardEnvelope;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::init_logger_with_filter;
use actor_playgroud::common::actor_sharding_setting;
use actor_playgroud::common::handoff_player::HandoffPlayer;
use actor_playgroud::common::hello::Hello;
use actor_playgroud::common::player_actor::player_actor_builder;
use actor_playgroud::common::player_message_extractor::PlayerMessageExtractor;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "mikai233")]
    system_name: String,
    #[arg(short, long)]
    addr: SocketAddrV4,
    #[arg(short, long, default_value = "127.0.0.1:2379")]
    etcd: SocketAddrV4,
    #[arg(short, long)]
    start_entity: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args { system_name, addr, etcd, start_entity } = Args::try_parse()?;
    init_logger_with_filter("debug,actor=debug,actor_core::actor::scheduler=info,actor_remote::remote_watcher=info,h2=info,tower=info,hyper=info");
    let client = Client::connect([etcd.to_string()], None).await?;
    let setting = actor_sharding_setting(addr, client)?;
    let system = ActorSystem::new(system_name, setting)?;
    system.register_extension(|system| {
        ClusterSharding::new_with_default_config(system)
    })?;
    let builder = player_actor_builder();
    let settings = ClusterShardingSettings::create(&system);
    let strategy = LeastShardAllocationStrategy::new(&system, 1, 1.0);
    let player_shard_region = ClusterSharding::get(&system)
        .start(
            "player",
            builder,
            settings.into(),
            PlayerMessageExtractor,
            strategy,
            HandoffPlayer,
        ).await?;
    let handle = if start_entity {
        let handle = tokio::spawn(async move {
            let mut index = 1;
            let mut players = vec![];
            for _ in 0..10 {
                players.push(random::<u64>());
            }
            loop {
                for id in &players {
                    let mut data = vec![];
                    for _ in 0..1048576 {
                        data.push(random());
                    }
                    let hello = Hello {
                        index,
                        data,
                    };
                    player_shard_region.cast_ns(ShardEnvelope::new(id.to_string(), hello));
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    index += 1;
                }
            }
        });
        Some(handle)
    } else {
        None
    };
    system.await?;
    if let Some(handle) = handle {
        handle.abort();
    }
    Ok(())
}