use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Message, derive_more::Display, derive_more::Deref)]
pub(crate) struct RouteeTerminated(ActorRef);
