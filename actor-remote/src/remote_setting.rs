use typed_builder::TypedBuilder;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::message::message_registration::MessageRegistration;

use crate::config::RemoteConfig;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RemoteSetting {
    pub system: ActorSystem,
    pub config: RemoteConfig,
    pub reg: MessageRegistration,
}