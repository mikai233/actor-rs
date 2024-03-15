use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

use bincode::{Decode, Encode};
use dyn_clone::DynClone;

use actor_core::{Actor, DynMessage, Message};
use actor_core::ext::type_name_of;
use actor_core::message::message_registration::IDPacket;

use crate::shard_region::{EntityId, ShardId};

pub trait MessageExtractor: Send + Sync + DynClone + Debug {
    fn entity_id(&self, message: &crate::ShardEnvelope) -> EntityId;

    fn shard_id(&self, message: &crate::ShardEnvelope) -> ShardId;

    fn unwrap_message(&self, message: crate::ShardEnvelope) -> DynMessage {
        message.message
    }
}

dyn_clone::clone_trait_object!(MessageExtractor);

#[derive(Debug)]
pub struct ShardEnvelope<A> where A: Actor {
    pub entity_id: EntityId,
    pub message: DynMessage,
    pub(crate) _phantom: PhantomData<A>,
}

impl<A> ShardEnvelope<A> where A: Actor {
    pub fn new<M>(entity_id: impl Into<EntityId>, message: M) -> Self where M: Message {
        let id = entity_id.into();
        Self {
            entity_id: id,
            message: DynMessage::user(message),
            _phantom: Default::default(),
        }
    }
}

impl<A> Display for ShardEnvelope<A> where A: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let actor_name = type_name_of::<A>();
        write!(f, "ShardEnvelope<{}> {{ entity_id: {}, message: {} }}", actor_name, self.entity_id, self.message.name())
    }
}

#[derive(Debug, Encode, Decode)]
pub(crate) struct CodecShardEnvelope {
    pub(crate) entity_id: EntityId,
    pub(crate) packet: IDPacket,
}
