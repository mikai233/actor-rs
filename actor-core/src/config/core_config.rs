use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use actor_derive::AsAny;

use crate::actor::coordinated_shutdown::Phase;
use crate::config::Config;
use crate::config::mailbox::Mailbox;

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsAny)]
#[serde(default)]
pub struct CoreConfig {
    pub mailbox: HashMap<String, Mailbox>,
    pub phases: HashMap<String, Phase>,
}

impl Config for CoreConfig {
    fn with_fallback(&self, other: Self) -> Self {
        let CoreConfig { mut mailbox, mut phases } = other;
        mailbox.extend(self.mailbox.clone());
        phases.extend(self.phases.clone());
        Self {
            mailbox,
            phases,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::actor::coordinated_shutdown::{Phase, PHASE_BEFORE_SERVICE_UNBIND, PHASE_SERVICE_UNBIND};
    use crate::config::core_config::CoreConfig;
    use crate::config::mailbox::Mailbox;

    #[test]
    fn test_config() -> anyhow::Result<()> {
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