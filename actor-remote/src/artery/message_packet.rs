use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_more::Constructor)]
pub(crate) struct MessagePacket {
    pub message_bytes: Vec<u8>,
    pub sender: Option<String>,
    pub target: String,
}

impl Display for MessagePacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessagePacket")
            .field("sender", &self.sender)
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}
