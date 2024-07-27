use imstr::ImString;
use serde::{Deserialize, Serialize};

use crate::config::debug::Debug;
use crate::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub provider: ImString,
    #[serde(rename = "guardian-supervisor-strategy")]
    pub guardian_supervisor_strategy: ImString,
    #[serde(rename = "creation-timeout")]
    pub creation_timeout: ConfigDuration,
    pub debug: Debug,
}