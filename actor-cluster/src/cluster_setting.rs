use std::fmt::Debug;

use actor_core::message::codec::MessageRegistry;

use crate::config::ClusterConfig;

#[derive(Debug, Clone)]
pub struct ClusterSetting {
    pub config: ClusterConfig,
    pub reg: MessageRegistry,
}
