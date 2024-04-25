use actor_core::message::message_registry::MessageRegistry;

use crate::config::RemoteConfig;

#[derive(Debug, Clone)]
pub struct RemoteSetting {
    pub config: RemoteConfig,
    pub reg: MessageRegistry,
}