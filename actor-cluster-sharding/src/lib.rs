use actor_core::message::message_registration::MessageRegistration;

use crate::shard::Shard;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::graceful_shutdown_req::GracefulShutdownReq;
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

pub(crate) const CLUSTER_SHARDING_CONFIG_NAME: &'static str = "cluster-sharding.toml";
pub(crate) const CLUSTER_SHARDING_CONFIG: &'static str = include_str!("../cluster-sharding.toml");

pub mod shard_region;
pub mod cluster_sharding;
mod cluster_sharding_guardian;
pub mod config;
pub mod shard_coordinator;
pub mod cluster_sharding_settings;
pub mod message_extractor;
pub(crate) mod shard;
mod entity_passivation_strategy;
mod handoff_stopper;
pub mod shard_allocation_strategy;

pub type ShardEnvelope = message_extractor::ShardEnvelope<ShardRegion>;

pub fn register_sharding(reg: &mut MessageRegistration) {
    reg.register_system::<GetShardHome>();
    reg.register_system::<GracefulShutdownReq>();
    reg.register_system::<RegionStopped>();
    reg.register_system::<Register>();
    reg.register_system::<RegisterProxy>();
    reg.register_system::<ShardStarted>();
    reg.register_system::<TerminateCoordinator>();
    reg.register_system::<BeginHandoff>();
    reg.register_system::<Handoff>();
    reg.register_system::<HostShard>();
    reg.register_system::<RegisterAck>();
    reg.register_system::<ShardHome>();
    reg.register_system::<ShardHomes>();
    reg.register_system::<message_extractor::ShardEnvelope<ShardRegion>>();
    reg.register_system::<message_extractor::ShardEnvelope<Shard>>();
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
