use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::time::Duration;

use clap::Parser;
use etcd_client::Client;
use rand::random;

use actor_cluster::cluster_provider::ClusterProviderBuilder;
use actor_cluster::config::ClusterConfig;
use actor_cluster_sharding::{register_sharding, ShardEnvelope};
use actor_cluster_sharding::cluster_sharding::ClusterSharding;
use actor_cluster_sharding::cluster_sharding_settings::ClusterShardingSettings;
use actor_cluster_sharding::config::ClusterShardingConfig;
use actor_cluster_sharding::shard_allocation_strategy::least_shard_allocation_strategy::LeastShardAllocationStrategy;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor_ref::ActorRefExt;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger_with_filter;
use actor_core::ext::message_ext::UserMessageExt;
use actor_playgroud::common::handoff_player::HandoffPlayer;
use actor_playgroud::common::hello::Hello;
use actor_playgroud::common::player_actor::PlayerActor;
use actor_playgroud::common::player_message_extractor::PlayerMessageExtractor;
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

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
async fn main() -> eyre::Result<()> {
    let Args { system_name, addr, etcd, start_entity } = Args::try_parse()?;
    init_logger_with_filter("debug,actor=debug,actor_core::actor::scheduler=info,actor_remote::remote_watcher=info,h2=info,tower=info,hyper=info");
    let client = Client::connect([etcd.to_string()], None).await?;
    let setting = ActorSetting::builder()
        .provider_fn(move |system| {
            let config = ClusterConfig {
                remote: RemoteConfig { transport: Transport::tcp(addr, None) },
                roles: HashSet::new(),
            };
            ClusterProviderBuilder::new()
                .client(client.clone())
                .register::<Hello>()
                .register_all(register_sharding)
                .config(config)
                .build(system.clone())
        })
        .build();
    let system = ActorSystem::new(system_name, setting)?;
    system.register_extension(|system| {
        ClusterSharding::new(system, ClusterShardingConfig::default())
    })?;
    let builder = PropsBuilderSync::new::<PlayerActor, _>(|id| {
        Props::new(|| {
            let player = PlayerActor {
                id,
                count: 0,
                start: None,
            };
            Ok(player)
        })
    });
    let settings = ClusterShardingSettings::create(&system);
    let strategy = LeastShardAllocationStrategy::new(&system, 1, 1.0);
    let player_shard_region = ClusterSharding::get(&system).start("player", builder, settings.into(), PlayerMessageExtractor, strategy, HandoffPlayer.into_dyn()).await?;
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