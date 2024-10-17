use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRef;

use crate::artery::remote_packet::RemotePacket;

#[derive(Debug, derive_more::Display)]
#[display("RemoteEnvelope {{ packet: {packet}, sender: {sender}, target: {target} }}")]
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
