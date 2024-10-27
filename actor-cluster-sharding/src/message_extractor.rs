use std::any::type_name;
use std::fmt::{Debug, Display, Formatter};

use actor_core::actor::actor_system::ActorSystem;
use actor_core::message::{DynMessage, Message};
use actor_core::Message;
use dyn_clone::DynClone;

use crate::shard_region::{EntityId, ShardId};
use actor_remote::codec::{MessageCodec, MessageCodecRegistry};
use serde::{Deserialize, Serialize};

pub trait MessageExtractor: Send + Sync + DynClone + Debug + Display {
    fn entity_id(&self, message: &ShardEnvelope) -> EntityId;

    fn shard_id(&self, message: &ShardEnvelope) -> ShardId;

    fn unwrap_message(&self, message: ShardEnvelope) -> DynMessage {
        message.message
    }
}

dyn_clone::clone_trait_object!(MessageExtractor);

#[derive(Debug, Message)]
pub struct ShardEnvelope {
    pub entity_id: EntityId,
    pub message: DynMessage,
}

impl ShardEnvelope {
    pub fn new<M>(entity_id: impl Into<EntityId>, message: M) -> Self
    where
        M: Message + MessageCodec,
    {
        let id = entity_id.into();
        Self {
            entity_id: id,
            message: Box::new(message),
        }
    }

    pub fn into_inner(self) -> DynMessage {
        self.message
    }
}

impl Display for ShardEnvelope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let actor_name = type_name::<A>();
        write!(
            f,
            "ShardEnvelope<{}> {{ entity_id: {}, message: {} }}",
            actor_name, self.entity_id, self.message
        )
    }
}

#[derive(Debug, Serialize, Deserialize, derive_more::Constructor)]
pub(crate) struct ShardEnvelopePacket {
    pub(crate) entity_id: Vec<u8>,
    pub(crate) message: Vec<u8>,
}

impl MessageCodec for ShardEnvelope {
    type M = ShardEnvelope;

    fn encode(
        message: &Self::M,
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Vec<u8>> {
        let Self { entity_id, message } = message;
        let entity_id = bincode::serialize(entity_id)?;
        let message = registry.encode(&message, system)?;
        let packet = bincode::serialize(&ShardEnvelopePacket::new(entity_id, message))?;
        Ok(packet)
    }

    fn decode(
        bytes: &[u8],
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Self::M> {
        let packet = bincode::deserialize::<ShardEnvelopePacket>(bytes)?;
        let entity_id: EntityId = bincode::deserialize(&packet.entity_id)?;
        let message = registry.decode(&packet.message, system)?;
        Ok(ShardEnvelope::new(entity_id, message))
    }
}
