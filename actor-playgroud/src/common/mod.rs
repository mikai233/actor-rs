use std::net::SocketAddrV4;

use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::cluster_setting::ClusterSetting;
use actor_cluster::config::ClusterConfig;
use actor_cluster_sharding::register_sharding;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::config::ConfigBuilder;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::message::codec::MessageRegistry;
use actor_remote::config::message_buffer::MessageBuffer;
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;

use crate::common::ask_ans::{MessageToAns, MessageToAsk};
use crate::common::greet::Greet;
use crate::common::hello::Hello;
use crate::common::init::Init;
use crate::common::test_message::TestMessage;

pub mod player_actor;
pub mod player_message_extractor;
pub mod hello;
pub mod handoff_player;
pub mod init;
pub mod ask_ans;
pub mod test_message;
pub mod greet;
pub mod singleton_actor;
pub mod stop_singleton;

pub fn build_cluster_setting(addr: SocketAddrV4, client: impl Into<EtcdClient>) -> anyhow::Result<ActorSetting> {
    let client = client.into();
    let config = ClusterConfig {
        remote: RemoteConfig { transport: Transport::tcp(addr, MessageBuffer::default()) },
        roles: Default::default(),
    };
    let mut reg = MessageRegistry::new();
    reg.register_user::<MessageToAsk>();
    reg.register_user::<MessageToAns>();
    reg.register_user::<TestMessage>();
    reg.register_user::<Greet>();
    let cluster_setting = ClusterSetting { config, reg, client };
    ActorSetting::new_with_default_config(ClusterActorRefProvider::builder(cluster_setting))
}

pub fn actor_sharding_setting(addr: SocketAddrV4, client: impl Into<EtcdClient>) -> anyhow::Result<ActorSetting> {
    let client = client.into();
    let mut config = ClusterConfig::builder().build()?;
    config.remote.transport = Transport::tcp(addr, MessageBuffer::default());
    let mut reg = MessageRegistry::new();
    register_sharding(&mut reg);
    reg.register_user::<Init>();
    reg.register_user::<Hello>();
    let cluster_setting = ClusterSetting { config, reg, client };
    ActorSetting::new_with_default_config(ClusterActorRefProvider::builder(cluster_setting))
}