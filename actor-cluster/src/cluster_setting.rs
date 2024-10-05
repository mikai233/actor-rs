use std::fmt::Debug;

use actor_remote::codec::MessageRegistry;

use crate::config::ClusterConfig;

#[derive(Debug, Clone)]
pub struct ClusterSetting {
    pub config: ClusterConfig,
    pub reg: MessageRegistry,
}
