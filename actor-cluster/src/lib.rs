mod cluster;
mod cluster_daemon;
pub mod cluster_provider;
pub mod cluster_setting;
mod cluster_singleton_manager;
mod region;
mod shard_region;
mod unique_address;
mod message;
pub mod ewatcher;
mod cluster_heartbeat;
mod cluster_event;
mod distributed_pub_sub;
mod member;
mod cluster_state;

#[test]
fn test_compile() {}