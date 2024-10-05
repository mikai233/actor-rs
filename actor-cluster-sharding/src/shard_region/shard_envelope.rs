use std::any::{Any, type_name};

use async_trait::async_trait;
use tracing::debug;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::ext::{decode_bytes, encode_bytes};
use actor_remote::codec::MessageRegistry;
use actor_core::message::MessageDecoder;

use crate::message_extractor::{CodecShardEnvelope, ShardEnvelope};
use crate::shard::Shard;
use crate::shard_region::ShardRegion;

#[async_trait]
impl Message for ShardEnvelope<ShardRegion> {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.graceful_shutdown_in_progress {
            debug!("{}: Ignore {}[{}] when ShardRegion is graceful shutdown in progress", actor.type_name, type_name::<Self>(),self.message.name());
        } else {
            actor.deliver_message(context, *self)?;
        }
        Ok(())
    }
}

impl CodecMessage for ShardEnvelope<ShardRegion> {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        #[derive(Clone)]
        struct D;
        impl MessageDecoder for D {
            fn decode(&self, bytes: &[u8], reg: &MessageRegistry) -> anyhow::Result<DynMessage> {
                let CodecShardEnvelope { entity_id, packet } = decode_bytes::<CodecShardEnvelope>(bytes)?;
                let message = reg.decode(packet)?;
                let message = ShardEnvelope::<ShardRegion> {
                    entity_id,
                    message,
                    _phantom: Default::default(),
                };
                Ok(DynMessage::user(message))
            }
        }

        Some(Box::new(D))
    }

    fn encode(self: Box<Self>, reg: &MessageRegistry) -> anyhow::Result<Vec<u8>> {
        let ShardEnvelope { entity_id, message, .. } = *self;
        let packet = reg.encode_boxed(message)?;
        let message = CodecShardEnvelope {
            entity_id,
            packet,
        };
        encode_bytes(&message)
    }

    fn clone_box(&self) -> anyhow::Result<Box<dyn CodecMessage>> {
        let envelope = Self {
            entity_id: self.entity_id.clone(),
            message: self.message.dyn_clone()?,
            _phantom: Default::default(),
        };
        Ok(Box::new(envelope))
    }

    fn cloneable(&self) -> bool {
        self.message.cloneable()
    }

    fn into_dyn(self) -> DynMessage {
        DynMessage::user(self)
    }
}

impl ShardEnvelope<ShardRegion> {
    pub(crate) fn into_shard_envelope(self) -> ShardEnvelope<Shard> {
        let Self { entity_id, message, .. } = self;
        ShardEnvelope::<Shard> {
            entity_id,
            message,
            _phantom: Default::default(),
        }
    }
}