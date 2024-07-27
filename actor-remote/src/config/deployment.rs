use imstr::ImString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    #[serde(rename = "enable-allow-list")]
    pub enable_allow_list: bool,
    #[serde(rename = "allowed-actor-classes")]
    pub allowed_actor_classes: Vec<ImString>,
}