use ahash::HashMap;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use crate::config::phase::Phase;
use crate::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatedShutdown {
    #[serde(rename = "default-phase-timeout")]
    pub default_phase_timeout: ConfigDuration,
    #[serde(rename = "terminate-actor-system")]
    pub terminate_actor_system: bool,
    pub phases: HashMap<ImString, Phase>,
}