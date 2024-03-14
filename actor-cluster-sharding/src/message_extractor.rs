use std::fmt::{Debug, Display, Formatter};

use dyn_clone::DynClone;

use actor_core::{DynMessage, Message};
use actor_derive::EmptyCodec;

use crate::shard_region::{EntityId, ShardId};

pub trait MessageExtractor: Send + Sync + DynClone + Debug {
    fn entity_id(&self, message: &ShardEntityEnvelope) -> EntityId;

    fn shard_id(&self, message: &ShardEntityEnvelope) -> ShardId;

    fn unwrap_message(&self, message: ShardEntityEnvelope) -> DynMessage {
        message.message
    }
}

dyn_clone::clone_trait_object!(MessageExtractor);

#[derive(Debug, EmptyCodec)]
pub struct ShardEntityEnvelope {
    pub entity_id: EntityId,
    pub message: DynMessage,
}

impl Display for ShardEntityEnvelope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShardingEnvelope {{ entity_id: {}, message: {} }}", self.entity_id, self.message.name())
    }
}

impl ShardEntityEnvelope {
    pub fn new<M>(id: impl Into<EntityId>, message: M) -> Self where M: Message {
        let id = id.into();
        Self {
            entity_id: id,
            message: DynMessage::user(message),
        }
    }
}