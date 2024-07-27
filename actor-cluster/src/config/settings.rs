use config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {}


impl Settings {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let settings: Self = config.get("akka.cluster")?;
        Ok(settings)
    }
}