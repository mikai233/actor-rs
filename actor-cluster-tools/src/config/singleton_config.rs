use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingletonConfig {
    pub singleton_name: String,
}