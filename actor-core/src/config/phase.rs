use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::duration::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Constructor)]
#[serde(default)]
pub struct Phase {
    pub depends_on: HashSet<String>,
    pub timeout: Option<Duration>,
    pub enabled: bool,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            depends_on: HashSet::new(),
            timeout: Some(Duration::from_secs(10)),
            enabled: true,
        }
    }
}
