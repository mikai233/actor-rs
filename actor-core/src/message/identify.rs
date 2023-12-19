use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::{Actor, CodecMessage, DynMessage, SystemMessage};
use crate::actor::actor_ref::{ActorRef, ActorRefSystemExt};
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::{ActorContext, Context};
use crate::actor::decoder::MessageDecoder;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let actor_identify = ActorIdentify {
            actor_ref: myself,
        };
        context.sender().foreach(|sender| {
            sender.cast_system(actor_identify, None);
        });
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ActorIdentify {
    pub(crate) actor_ref: ActorRef,
}

impl CodecMessage for ActorIdentify {
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
            fn decode(&self, provider: &ActorRefProvider, bytes: &[u8]) -> anyhow::Result<DynMessage> {
                let serialized: SerializedActorRef = decode_bytes(bytes)?;
                let actor_ref = provider.resolve_actor_ref(&serialized.path);
                let message = SystemDelegate::new(ActorIdentify { actor_ref });
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized: SerializedActorRef = self.actor_ref.clone().into();
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

#[async_trait]
impl SystemMessage for ActorIdentify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        todo!()
    }
}