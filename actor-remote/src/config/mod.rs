use config::Source;
use serde::{Deserialize, Serialize};

use actor_core::AsAny;
use actor_core::config::Config;
use actor_core::message::codec::MessageRegistry;

use crate::config::settings::Settings;

pub mod message_buffer;
pub mod settings;
pub mod watch_failure_detector;
pub mod deployment;
pub mod artery;
pub mod advanced;

#[derive(Debug, Clone, AsAny)]
pub struct RemoteConfig {
    pub settings: Settings,
    pub registry: MessageRegistry,
}

impl Config for RemoteConfig {}

impl RemoteConfig {
    pub fn new(config: &config::Config, registry: MessageRegistry) -> anyhow::Result<Self> {
        let settings = Settings::new(config)?;
        Ok(Self {
            settings,
            registry,
        })
    }
}