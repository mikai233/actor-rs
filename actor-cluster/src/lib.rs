pub use etcd_client;
pub use parking_lot;

pub(crate) const REFERENCE: &'static str = include_str!("../reference.toml");

pub mod cluster;
mod cluster_core_daemon;
pub(crate) mod cluster_core_supervisor;
mod cluster_daemon;
pub mod cluster_event;
pub mod cluster_provider;
pub mod cluster_setting;
mod cluster_settings;
pub mod cluster_state;
pub mod config;
mod coordinated_shutdown_leave;
pub mod downing_provider;
pub mod etcd_actor;
mod gossip;
mod heartbeat;
pub mod member;
pub(crate) mod membership_state;
mod on_member_status_changed_listener;
mod reachability;
pub mod unique_address;
mod vector_clock;
pub(crate) mod cluster_read_view;
