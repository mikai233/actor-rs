use actor_remote::codec::{register_message, MessageCodecRegistry};
use heartbeat::{
    cluster_heartbeat_receiver::heartbeat::Heartbeat,
    cluster_heartbeat_sender::heartbeat_rsp::HeartbeatRsp,
};
pub use parking_lot;

pub(crate) const REFERENCE: &'static str = include_str!("../reference.json");

pub mod cluster;
mod cluster_core_daemon;
pub(crate) mod cluster_core_supervisor;
mod cluster_daemon;
pub mod cluster_event;
pub mod cluster_provider;
pub(crate) mod cluster_read_view;
pub mod cluster_setting;
mod cluster_settings;
pub mod cluster_state;
pub mod config;
mod coordinated_shutdown_leave;
pub mod downing_provider;
mod gossip;
mod heartbeat;
pub mod member;
pub(crate) mod membership_state;
mod on_member_status_changed_listener;
mod reachability;
pub mod unique_address;
mod vector_clock;

pub fn register_remote_system_message(registry: &mut dyn MessageCodecRegistry) {
    register_message::<Heartbeat>(registry);
    register_message::<HeartbeatRsp>(registry);
}
