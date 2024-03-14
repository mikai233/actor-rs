use std::any::Any;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use bincode::error::{DecodeError, EncodeError};

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::{decode_bytes, encode_bytes};
use actor_core::message::message_registration::{IDPacket, MessageRegistration};
use actor_core::message::MessageDecoder;

use crate::message_extractor::ShardEntityEnvelope;
use crate::shard::Shard;
use crate::shard_region::EntityId;

#[derive(Debug)]
pub(crate) struct ShardEnvelope(pub(crate) ShardEntityEnvelope);

#[async_trait]
impl Message for ShardEnvelope {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.deliver_message(context, *self, context.sender().cloned())?;
        Ok(())
    }
}


#[derive(Debug, Encode, Decode)]
struct CodecShardEnvelope {
    entity_id: EntityId,
    packet: IDPacket,
}

impl CodecMessage for ShardEnvelope {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        #[derive(Clone)]
        struct D;
        impl MessageDecoder for D {
            fn decode(&self, bytes: &[u8], reg: &MessageRegistration) -> Result<DynMessage, DecodeError> {
                let CodecShardEnvelope { entity_id, packet } = decode_bytes::<CodecShardEnvelope>(bytes)?;
                let message = reg.decode(packet)?;
                let message = ShardEntityEnvelope {
                    entity_id,
                    message,
                };
                Ok(DynMessage::user(ShardEnvelope(message)))
            }
        }

        Some(Box::new(D))
    }

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        let ShardEntityEnvelope { entity_id, message } = &self.0;
        let packet = reg.encode_boxed(message)?;
        let message = CodecShardEnvelope {
            entity_id: entity_id.clone(),
            packet,
        };
        encode_bytes(&message)
    }

    fn dyn_clone(&self) -> anyhow::Result<DynMessage> {
        self.0.dyn_clone().map(|m| {
            let message = ShardEntityEnvelope {
                entity_id: self.0.entity_id.clone(),
                message: m,
            };
            DynMessage::user(ShardEnvelope(message))
        })
    }

    fn is_cloneable(&self) -> bool {
        self.0.message.is_cloneable()
    }
}