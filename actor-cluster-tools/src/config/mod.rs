mod singleton_config;

use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ToolsConfig {

}