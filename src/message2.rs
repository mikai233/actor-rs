use std::borrow::Cow;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ActorRemoteMessage {
    User {
        name: Cow<'static, str>,
        message: Vec<u8>,
    },
    System {
        name: Cow<'static, str>,
        message: Vec<u8>,
    },
}

impl ActorRemoteMessage {
    pub fn name(&self) -> &str {
        match self {
            ActorRemoteMessage::User { name, .. } => name,
            ActorRemoteMessage::System { name, .. } => name,
        }
    }
}