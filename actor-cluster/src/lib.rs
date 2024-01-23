pub const ACTOR_CLUSTER_CONFIG: &'static str = include_str!("../cluster.toml");

mod cluster;
mod cluster_daemon;
pub mod cluster_provider;
pub mod cluster_setting;
mod cluster_singleton_manager;
mod region;
mod unique_address;
mod message;
pub mod key_watcher;
mod cluster_heartbeat;
mod cluster_event;
mod distributed_pub_sub;
mod member;
mod cluster_state;
mod lease_keeper;
pub mod cluster_config;