use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::{AsyncMessage, Message, SystemMessage, UntypedMessage};
use crate::actor::actor_path::ActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::cell::ActorCell;

pub(crate) mod dead_letter_ref;
pub mod local_ref;
pub(crate) mod virtual_path_container;
pub(crate) mod deferred_ref;
pub(crate) mod function_ref;
pub(crate) mod routed_actor_ref;


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            path: self.path().to_serialization(),
        }
    }
}

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>>;
    fn get_single_child(&self, name: &String) -> Option<ActorRef>;
}
