use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Debug {
    pub receive: bool,
    #[serde(rename = "auto-receive")]
    pub auto_receive: bool,
    pub lifecycle: bool,
    pub fsm: bool,
    #[serde(rename = "event-stream")]
    pub event_stream: bool,
    pub unwatch: bool,
    #[serde(rename = "router-misconfiguration")]
    pub router_misconfiguration: bool,
}