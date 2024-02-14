use std::fmt::Debug;

use dyn_clone::DynClone;

use actor_core::DynMessage;

use crate::shard_region::ShardId;

pub trait MessageExtractor: Send + Sync + DynClone + Debug {
    fn entity_id(&self) -> String;

    fn entity_message(&self, message: DynMessage) -> DynMessage;

    fn shard_id(&self, message: DynMessage) -> ShardId;
}

dyn_clone::clone_trait_object!(MessageExtractor);