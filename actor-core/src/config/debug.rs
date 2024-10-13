use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct DebugConfig {
    pub receive: bool,
    pub auto_receive: bool,
    pub lifecycle: bool,
    pub fsm: bool,
    pub event_stream: bool,
    pub unwatch: bool,
    pub router_misconfiguration: bool,
}
