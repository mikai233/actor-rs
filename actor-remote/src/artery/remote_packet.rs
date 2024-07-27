use bincode::{Decode, Encode};

use actor_core::message::message_registry::IDPacket;

#[derive(Debug, Encode, Decode)]
pub(crate) struct RemotePacket {
    pub packet: IDPacket,
    pub sender: Option<String>,
    pub target: String,
}