use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use actor_core::actor::config::Config;
use actor_core::actor::coordinated_shutdown::Phase;
use actor_derive::AsAny;

#[derive(Debug, Clone, Default, AsAny, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterConfig {
    pub phases: HashMap<String, Phase>,
}

impl Config for ClusterConfig {
    fn with_fallback(&self, fallback: Box<dyn Config>) -> anyhow::Result<Box<dyn Config>> {
        let fallback_config = fallback.into_any().downcast::<ClusterConfig>().map_err(|_| anyhow!("cannot downcast Box<dyn Config> to ClusterConfig"))?;
        let mut phases = self.phases.clone();
        let fallback_phases = fallback_config.phases;
        for (name, phase) in fallback_phases {
            phases.entry(name).or_insert(phase);
        }
        let config = ClusterConfig {
            phases,
        };
        Ok(Box::new(config))
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