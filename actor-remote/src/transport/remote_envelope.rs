use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRef;
use actor_core::message::message_registry::IDPacket;

use crate::transport::remote_packet::RemotePacket;

#[derive(Debug)]
pub(crate) struct RemoteEnvelope {
    pub packet: IDPacket,
    pub sender: Option<ActorRef>,
    pub target: ActorRef,
}

impl Into<RemotePacket> for RemoteEnvelope {
    fn into(self) -> RemotePacket {
        RemotePacket {
            packet: self.packet,
            sender: self.sender.map(|s| s.path().to_serialization_format()),
            target: self.target.path().to_serialization_format(),
        }
    }
}