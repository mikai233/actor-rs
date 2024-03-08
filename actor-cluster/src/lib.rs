pub(crate) const CLUSTER_CONFIG_NAME: &'static str = "cluster.toml";
pub(crate) const CLUSTER_CONFIG: &'static str = include_str!("../cluster.toml");

pub mod cluster;
mod cluster_daemon;
pub mod cluster_provider;
pub mod cluster_setting;
pub mod unique_address;
mod message;
pub mod etcd_watcher;
pub mod cluster_event;
pub mod member;
mod cluster_state;
pub mod lease_keeper;
pub mod config;
mod on_member_status_changed_listener;
mod heartbeat;
pub mod etcd_actor;