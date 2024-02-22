use std::fmt::{Debug, Display, Formatter};

use dyn_clone::DynClone;

use actor_core::DynMessage;
use actor_derive::EmptyCodec;

use crate::shard_region::{EntityId, ShardId};

pub trait MessageExtractor: Send + Sync + DynClone + Debug {
    fn entity_id(&self, message: &ShardingEnvelope) -> String;

    fn shard_id(&self, entity_id: &EntityId) -> ShardId;

    fn unwrap_message(&self, message: ShardingEnvelope) -> DynMessage;
}

dyn_clone::clone_trait_object!(MessageExtractor);

#[derive(Debug, EmptyCodec)]
pub struct ShardingEnvelope {
    pub entity_id: EntityId,
    pub message: DynMessage,
}

impl Display for ShardingEnvelope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShardingEnvelope {{ entity_id: {}, message: {} }}", self.entity_id, self.message.name)
    }
}