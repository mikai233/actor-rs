use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::actor::config::Config;
use crate::actor::config::mailbox::Mailbox;
use crate::actor::coordinated_shutdown::Phase;

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


