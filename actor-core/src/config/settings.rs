use ahash::HashMap;
use config::Config;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use crate::config::actor::Actor;
use crate::config::circuit_breaker::CircuitBreaker;
use crate::config::coordinated_shutdown::CoordinatedShutdown;
use crate::config::mailbox::Mailbox;
use crate::util::duration::ConfigDuration;
use crate::util::opt_config::OptConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "log-config-on-start")]
    pub log_config_on_start: bool,
    #[serde(rename = "log-dead-letters")]
    pub log_dead_letters: bool,
    #[serde(rename = "log-dead-letters-during-shutdown")]
    pub log_dead_letters_during_shutdown: OptConfig<bool>,
    #[serde(rename = "log-dead-letters-suspend-duration")]
    pub log_dead_letters_suspend_duration: ConfigDuration,
    pub extensions: Vec<String>,
    pub actor: Actor,
    pub mailbox: HashMap<ImString, Mailbox>,
    #[serde(rename = "coordinated-shutdown")]
    pub coordinated_shutdown: CoordinatedShutdown,
    #[serde(rename = "circuit-breaker")]
    pub circuit_breaker: HashMap<ImString, CircuitBreaker>,
}

impl Settings {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let settings: Self = config.get("akka.actor")?;
        Ok(settings)
    }
}
