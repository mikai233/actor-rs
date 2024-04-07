use actor_core::message::message_registration::MessageRegistration;

use crate::config::RemoteConfig;

#[derive(Debug, Clone)]
pub struct RemoteSetting {
    pub config: RemoteConfig,
    pub reg: MessageRegistration,
}