use std::net::SocketAddrV4;

use typed_builder::TypedBuilder;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::message::message_registration::MessageRegistration;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RemoteSetting {
    pub system: ActorSystem,
    pub addr: SocketAddrV4,
    pub reg: MessageRegistration,
}