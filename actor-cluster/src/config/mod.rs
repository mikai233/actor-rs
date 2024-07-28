use ahash::HashSet;
use serde::{Deserialize, Serialize};

use actor_core::AsAny;
use actor_core::config::Config;

use crate::config::settings::Settings;

pub mod settings;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterConfig {
    pub settings: Settings,
    pub roles: HashSet<String>,
}

impl Config for ClusterConfig {}

impl ClusterConfig {
}
