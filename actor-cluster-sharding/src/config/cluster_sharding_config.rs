use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsAny)]
pub struct ClusterShardingConfig {
    #[serde(default = "sharding")]
    pub guardian_name: String,
    pub role: Option<String>,
    
}