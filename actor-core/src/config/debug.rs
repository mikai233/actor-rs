use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct Debug {
    pub receive: bool,
    pub auto_receive: bool,
    pub lifecycle: bool,
    pub fsm: bool,
    pub event_stream: bool,
    pub unhandled: bool,
    pub router_misconfiguration: bool,
}
