use std::collections::HashMap;
use std::time::Duration;

use config::{File, FileFormat, Source};
use config::builder::DefaultState;
use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

use crate::actor::coordinated_shutdown::Phase;
use crate::config::{Config, ConfigBuilder};
use crate::config::mailbox::Mailbox;
use crate::CORE_CONFIG;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct CoreConfig {
    pub mailbox: HashMap<String, Mailbox>,
    pub phases: HashMap<String, Phase>,
    pub creation_timeout: Duration,
}

impl Config for CoreConfig {}

impl CoreConfig {
    pub fn builder() -> CoreConfigBuilder {
        CoreConfigBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct CoreConfigBuilder {
    builder: config::ConfigBuilder<DefaultState>,
}

impl ConfigBuilder for CoreConfigBuilder {
    type C = CoreConfig;

    fn add_source<T>(self, source: T) -> eyre::Result<Self> where T: Source + Send + Sync + 'static {
        Ok(Self { builder: self.builder.add_source(source) })
    }

    fn build(self) -> eyre::Result<Self::C> {
        let builder = self.builder.add_source(File::from_str(CORE_CONFIG, FileFormat::Toml));
        let core_config = builder.build()?.try_deserialize::<Self::C>()?;
        Ok(core_config)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::actor::coordinated_shutdown::{Phase, PHASE_BEFORE_SERVICE_UNBIND, PHASE_SERVICE_UNBIND};
    use crate::config::core_config::CoreConfig;
    use crate::config::mailbox::Mailbox;

    #[test]
    fn test_config() -> eyre::Result<()> {
        let mut config = CoreConfig::default();
        config.mailbox.insert(
            "default".to_string(),
            Mailbox {
                mailbox_capacity: 5000,
                stash_capacity: Some(5000),
                throughput: 50,
            },
        );
        config.phases.insert(
            PHASE_BEFORE_SERVICE_UNBIND.to_string(),
            Phase::default(),
        );
        let mut depends_on = HashSet::new();
        depends_on.insert(PHASE_BEFORE_SERVICE_UNBIND.to_string());
        config.phases.insert(
            PHASE_SERVICE_UNBIND.to_string(),
            Phase {
                depends_on,
                ..Default::default()
            },
        );
        println!("{}", toml::to_string(&config)?);
        Ok(())
    }
}