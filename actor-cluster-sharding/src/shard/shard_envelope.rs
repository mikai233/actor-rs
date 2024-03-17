use std::any::Any;

use async_trait::async_trait;
use bincode::error::{DecodeError, EncodeError};

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::{decode_bytes, encode_bytes};
use actor_core::message::message_registration::MessageRegistration;
use actor_core::message::MessageDecoder;

use crate::message_extractor::{CodecShardEnvelope, ShardEnvelope};
use crate::shard::Shard;
use crate::shard_region::ShardRegion;

#[async_trait]
impl Message for ShardEnvelope<Shard> {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.deliver_message(context, *self, context.sender().cloned())
    }
}

impl CodecMessage for ShardEnvelope<Shard> {
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
                let message = ShardEnvelope::<Shard> {
                    entity_id,
                    message,
                    _phantom: Default::default(),
                };
                Ok(DynMessage::user(message))
            }
        }

        Some(Box::new(D))
    }

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        let ShardEnvelope { entity_id, message, .. } = &self;
        let packet = reg.encode_boxed(message)?;
        let message = CodecShardEnvelope {
            entity_id: entity_id.clone(),
            packet,
        };
        encode_bytes(&message)
    }

    fn dyn_clone(&self) -> anyhow::Result<DynMessage> {
        self.message.dyn_clone().map(|m| {
            let message = ShardEnvelope::<Shard> {
                entity_id: self.entity_id.clone(),
                message: m,
                _phantom: Default::default(),
            };
            DynMessage::user(message)
        })
    }

    fn is_cloneable(&self) -> bool {
        self.message.is_cloneable()
    }
}

impl ShardEnvelope<Shard> {
    pub(crate) fn into_shard_region_envelope(self) -> ShardEnvelope<ShardRegion> {
        let Self { entity_id, message, .. } = self;
        ShardEnvelope::<ShardRegion> {
            entity_id,
            message,
            _phantom: Default::default(),
        }
    }
}