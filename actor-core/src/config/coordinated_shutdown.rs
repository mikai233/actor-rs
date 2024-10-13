use ahash::HashMap;
use serde::{Deserialize, Serialize};

use crate::config::phase::Phase;

use super::duration::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct CoordinatedShutdown {
    pub default_phase_timeout: Duration,
    pub terminate_actor_system: bool,
    pub phases: HashMap<String, Phase>,
}
