use std::collections::HashSet;

use imstr::ImString;
use serde::{Deserialize, Serialize};

use crate::util::duration::ConfigDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    pub depends_on: HashSet<ImString>,
    pub timeout: Option<ConfigDuration>,
    pub enabled: bool,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            depends_on: HashSet::new(),
            timeout: Some(ConfigDuration::from_secs(10)),
            enabled: true,
        }
    }
}