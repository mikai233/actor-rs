use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

mod singleton_config;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ToolsConfig {}