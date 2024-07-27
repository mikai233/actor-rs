use serde::{Deserialize, Serialize};

use crate::config::message_buffer::MessageBuffer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advanced {
    #[serde(rename = "maximum-frame-size")]
    pub maximum_frame_size: usize,
    #[serde(rename = "outbound-message-buffer")]
    pub outbound_message_buffer: MessageBuffer,
    #[serde(rename = "outbound-control-queue-size")]
    pub outbound_control_queue_size: usize,
    #[serde(rename = "system-message-buffer")]
    pub system_message_buffer: MessageBuffer,
}
