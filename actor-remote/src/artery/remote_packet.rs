use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RemotePacket {
    pub packet: IDPacket,
    pub sender: Option<String>,
    pub target: String,
}
