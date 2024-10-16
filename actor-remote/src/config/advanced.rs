use serde::{Deserialize, Serialize};

use crate::config::message_buffer::MessageBuffer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advanced {
    pub maximum_frame_size: usize,
    pub outbound_message_buffer: MessageBuffer,
    pub outbound_control_queue_size: usize,
    pub system_message_buffer: MessageBuffer,
}
