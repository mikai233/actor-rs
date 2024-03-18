use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use clap::Parser;
use etcd_client::Client;
use rand::random;
use tracing::info;

use actor_cluster::cluster_provider::ClusterProviderBuilder;
use actor_cluster::config::ClusterConfig;
use actor_cluster_sharding::{register_sharding, ShardEnvelope};
use actor_cluster_sharding::cluster_sharding::ClusterSharding;
use actor_cluster_sharding::cluster_sharding_settings::ClusterShardingSettings;
use actor_cluster_sharding::config::ClusterShardingConfig;
use actor_cluster_sharding::message_extractor::MessageExtractor;
use actor_cluster_sharding::shard_allocation_strategy::least_shard_allocation_strategy::LeastShardAllocationStrategy;
use actor_cluster_sharding::shard_region::{EntityId, ImShardId, ShardId};
use actor_core::{Actor, Message};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::ext::init_logger_with_filter;
use actor_core::ext::message_ext::UserMessageExt;
use actor_derive::{CMessageCodec, MessageCodec};
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

const SHARD_MOD: usize = 10000;

#[derive(Debug)]
struct PlayerActor {
    id: ImShardId,
}

#[async_trait]
impl Actor for PlayerActor {
    async fn started(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        info!("player {} started", self.id);
        Ok(())
    }
}

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
struct HandoffPlayer;

#[async_trait]
impl Message for HandoffPlayer {
    type A = PlayerActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("player {} handoff", actor.id);
        context.stop(context.myself());
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
struct Hello {
    index: i32,
    data: Vec<u8>,
}

#[async_trait]
impl Message for Hello {
    type A = PlayerActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("player {} {} receive hello {}",context.myself(), actor.id, self.index);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PlayerMessageExtractor;

impl MessageExtractor for PlayerMessageExtractor {
    fn entity_id(&self, message: &ShardEnvelope) -> EntityId {
        message.entity_id.clone()
    }

    fn shard_id(&self, message: &ShardEnvelope) -> ShardId {
        let entity_id = usize::from_str(&message.entity_id).unwrap();
        let shard = entity_id % SHARD_MOD;
        shard.to_string()
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "mikai233")]
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
    let args = Args::try_parse()?;
    init_logger_with_filter("debug,actor=debug,actor-core::scheduler=info,h2=info,tower=info,hyper=info");
    let client = Client::connect([args.etcd.to_string()], None).await?;
    let setting = ActorSetting::builder()
        .provider_fn(move |system| {
            let config = ClusterConfig {
                remote: RemoteConfig { transport: Transport::tcp(args.addr, None) },
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
    let system = ActorSystem::new("mikai233", setting)?;
    system.register_extension(|system| {
        ClusterSharding::new(system, ClusterShardingConfig::default())
    })?;
    let builder = PropsBuilderSync::new::<PlayerActor, _>(|id| {
        Props::new(|| {
            let player = PlayerActor {
                id
            };
            Ok(player)
        })
    });
    let settings = ClusterShardingSettings::create(&system);
    let strategy = LeastShardAllocationStrategy::new(&system, 1, 1.0);
    let player_shard_region = ClusterSharding::get(&system).start("player", builder, settings.into(), PlayerMessageExtractor, strategy, HandoffPlayer.into_dyn()).await?;
    let handle = if args.start_entity {
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
                    tokio::time::sleep(Duration::from_secs(5)).await;
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