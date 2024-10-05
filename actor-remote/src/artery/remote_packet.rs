use bincode::{Decode, Encode};

use crate::codec::IDPacket;

#[derive(Debug, Encode, Decode)]
pub(crate) struct RemotePacket {
    pub packet: IDPacket,
    pub sender: Option<String>,
    pub target: String,
}