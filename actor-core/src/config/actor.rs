use ahash::HashMap;
use serde::{Deserialize, Serialize};

use crate::config::debug::DebugConfig;
use crate::config::mailbox::Mailbox;

use super::duration::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct ActorConfig {
    pub guardian_supervisor_strategy: String,
    pub creation_timeout: Duration,
    pub mailbox: HashMap<String, Mailbox>,
    pub debug: DebugConfig,
}
