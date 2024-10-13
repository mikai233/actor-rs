use ahash::HashMap;
use serde::{Deserialize, Serialize};

use crate::config::actor::ActorConfig;
use crate::config::circuit_breaker::CircuitBreaker;
use crate::config::coordinated_shutdown::CoordinatedShutdown;

use super::duration::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct SettingsConfig {
    pub log_config_on_start: bool,
    pub log_dead_letters: bool,
    pub log_dead_letters_during_shutdown: Option<bool>,
    pub log_dead_letters_suspend_duration: Duration,
    pub extensions: Vec<String>,
    pub actor: ActorConfig,
    pub coordinated_shutdown: CoordinatedShutdown,
    pub circuit_breaker: HashMap<String, CircuitBreaker>,
}
