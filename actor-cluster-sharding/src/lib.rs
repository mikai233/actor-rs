use actor_remote::codec::{register_message, MessageCodecRegistry, MessageRegistry};
use message_extractor::ShardEnvelope;

use crate::shard::Shard;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::graceful_shutdown_req::GracefulShutdownReq;
use crate::shard_coordinator::rebalance_worker::begin_handoff_ack::BeginHandoffAck;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::region_stopped::RegionStopped;
use crate::shard_coordinator::register::Register;
use crate::shard_coordinator::register_proxy::RegisterProxy;
use crate::shard_coordinator::shard_started::ShardStarted;
use crate::shard_coordinator::terminate_coordinator::TerminateCoordinator;
use crate::shard_region::begin_handoff::BeginHandoff;
use crate::shard_region::handoff::Handoff;
use crate::shard_region::host_shard::HostShard;
use crate::shard_region::register_ack::RegisterAck;
use crate::shard_region::shard_home::ShardHome;
use crate::shard_region::shard_homes::ShardHomes;
use crate::shard_region::ShardRegion;

pub const CLUSTER_SHARDING_CONFIG: &'static str = include_str!("../cluster-sharding.toml");

pub mod cluster_sharding;
mod cluster_sharding_guardian;
pub mod cluster_sharding_settings;
pub mod config;
mod entity_passivation_strategy;
mod handoff_stopper;
pub mod message_extractor;
pub mod shard;
pub mod shard_allocation_strategy;
pub mod shard_coordinator;
pub mod shard_region;

pub fn register_cluster_sharding_message(registry: &mut dyn MessageCodecRegistry) {
    register_message::<GetShardHome>(registry);
    register_message::<GracefulShutdownReq>(registry);
    register_message::<RegionStopped>(registry);
    register_message::<Register>(registry);
    register_message::<RegisterProxy>(registry);
    register_message::<ShardStarted>(registry);
    register_message::<ShardStopped>(registry);
    register_message::<TerminateCoordinator>(registry);
    register_message::<BeginHandoff>(registry);
    register_message::<BeginHandoffAck>(registry);
    register_message::<Handoff>(registry);
    register_message::<HostShard>(registry);
    register_message::<RegisterAck>(registry);
    register_message::<ShardHome>(registry);
    register_message::<ShardHomes>(registry);
    register_message::<ShardEnvelope>(registry);
}
