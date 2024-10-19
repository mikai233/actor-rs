use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub enable_allow_list: bool,
    pub allowed_actor_classes: Vec<String>,
}
