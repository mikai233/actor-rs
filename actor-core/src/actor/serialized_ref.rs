use std::fmt::{Display, Formatter};

use bincode::{Decode, Encode};

use crate::actor::actor_path::ActorPath;
use crate::actor::actor_path::TActorPath;
use crate::actor::actor_ref::ActorRef;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode, )]
pub struct SerializedActorRef {
    pub path: String,
}

impl SerializedActorRef {
    pub fn parse_to_path(&self) -> anyhow::Result<ActorPath> {
        self.path.parse()
    }
}

impl Display for SerializedActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Into<SerializedActorRef> for ActorRef {
    fn into(self) -> SerializedActorRef {
        SerializedActorRef {
            path: self.path().to_serialization_format(),
        }
    }
}