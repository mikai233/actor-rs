use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::actor::coordinated_shutdown::Phase;
use crate::config::Config;
use crate::config::mailbox::Mailbox;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    pub mailbox: HashMap<String, Mailbox>,
    pub phases: HashMap<String, Phase>,
}

impl Config for CoreConfig {
    fn merge(&self, other: Self) -> Self {
        let CoreConfig { mailbox, phases } = other;
        let mut merged_mailbox = self.mailbox.clone();
        merged_mailbox.extend(mailbox);
        let mut merged_phases = self.phases.clone();
        merged_phases.extend(phases);
        Self {
            mailbox: merged_mailbox,
            phases: merged_phases,
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