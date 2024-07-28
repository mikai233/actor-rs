use ahash::HashMap;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use crate::config::debug::Debug;
use crate::config::mailbox::Mailbox;
use crate::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    #[serde(rename = "guardian-supervisor-strategy")]
    pub guardian_supervisor_strategy: ImString,
    #[serde(rename = "creation-timeout")]
    pub creation_timeout: ConfigDuration,
    pub mailbox: HashMap<ImString, Mailbox>,
    pub debug: Debug,
}