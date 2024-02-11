use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsAny)]
pub struct ClusterShardingConfig {
    #[serde(default = "default_guardian_name")]
    pub guardian_name: String,
    pub role: Option<String>,

}

fn default_guardian_name() -> String {
    "sharing".to_string()
}