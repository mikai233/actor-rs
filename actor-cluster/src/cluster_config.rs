use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use actor_core::actor::config::Config;
use actor_core::actor::coordinated_shutdown::Phase;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterConfig {
    pub phases: HashMap<String, Phase>,
}

impl Config for ClusterConfig {
    fn merge(&self, other: Self) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use actor_core::actor::coordinated_shutdown::{Phase, PHASE_BEFORE_SERVICE_UNBIND, PHASE_SERVICE_UNBIND};

    use crate::cluster_config::ClusterConfig;

    #[test]
    fn test_config() -> anyhow::Result<()> {
        let mut config = ClusterConfig::default();
        config.phases.insert(PHASE_BEFORE_SERVICE_UNBIND.to_string(), Phase::default());
        let mut phase = Phase::default();
        phase.depends_on.insert(PHASE_BEFORE_SERVICE_UNBIND.to_string());
        config.phases.insert(PHASE_SERVICE_UNBIND.to_string(), phase);
        let str = toml::to_string(&config)?;
        println!("{}", str);
        Ok(())
    }
}