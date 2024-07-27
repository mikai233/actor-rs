use std::net::SocketAddr;

use imstr::ImString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artery {
    pub enabled: bool,
    pub transport: Transport,
    pub canonical: SocketAddr,
    #[serde(rename = "untrusted-mode")]
    pub untrusted_mode: bool,
    #[serde(rename = "trusted-selection-paths")]
    pub trusted_selection_paths: Vec<ImString>,
    #[serde(rename = "log-received-messages")]
    pub log_received_messages: bool,
    #[serde(rename = "log-sent-messages")]
    pub log_sent_messages: bool,
    #[serde(rename = "log-frame-size-exceeding")]
    pub log_frame_size_exceeding: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Transport {
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "tls-tcp")]
    TlsTcp,
}