use actor_derive::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("PoisonPill")]
pub struct PoisonPill;
