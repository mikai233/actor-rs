use std::fmt::Debug;

use actor_core::ext::etcd_client::EtcdClient;
use actor_core::message::message_registry::MessageRegistry;

use crate::config::ClusterConfig;

#[derive(Debug, Clone)]
pub struct ClusterSetting {
    pub config: ClusterConfig,
    pub reg: MessageRegistry,
}
