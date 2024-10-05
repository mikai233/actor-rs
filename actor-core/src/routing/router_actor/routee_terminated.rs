use serde::{Deserialize, Serialize};

use crate::actor_ref::ActorRef;
use crate::Message;

#[derive(Debug, Serialize, Deserialize, Message, derive_more::Display)]
pub(crate) struct RouteeTerminated(ActorRef);
