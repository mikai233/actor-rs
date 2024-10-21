use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_more::Constructor)]
pub(crate) struct MessagePacket {
    pub msg: Vec<u8>,
    pub sender: Option<String>,
    pub target: String,
}
