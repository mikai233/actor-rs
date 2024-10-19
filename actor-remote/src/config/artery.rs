use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artery {
    pub enabled: bool,
    pub transport: Transport,
    pub canonical: SocketAddr,
    pub untrusted_mode: bool,
    pub trusted_selection_paths: Vec<String>,
    pub log_received_messages: bool,
    pub log_sent_messages: bool,
    pub log_frame_size_exceeding: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Transport {
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "tls-tcp")]
    TlsTcp,
}
