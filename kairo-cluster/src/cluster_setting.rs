use std::fmt::Debug;

use kairo_core::ext::etcd_client::EtcdClient;
use kairo_core::message::message_registry::MessageRegistry;

use crate::config::ClusterConfig;

#[derive(Debug, Clone)]
pub struct ClusterSetting {
    pub config: ClusterConfig,
    pub reg: MessageRegistry,
    pub client: EtcdClient,
}
