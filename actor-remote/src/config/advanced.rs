use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advanced {
    #[serde(rename = "maximum-frame-size")]
    pub maximum_frame_size: usize,
    #[serde(rename = "outbound-message-queue-size")]
    pub outbound_message_queue_size: usize,
    #[serde(rename = "outbound-control-queue-size")]
    pub outbound_control_queue_size: usize,
    #[serde(rename = "outbound-large-message-queue-size")]
    pub outbound_large_message_queue_size: usize,
    #[serde(rename = "system-message-buffer-size")]
    pub system_message_buffer_size: usize,
}
