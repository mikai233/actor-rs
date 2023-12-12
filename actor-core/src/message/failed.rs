use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Actor, CodecMessage, DynMessage, SystemMessage};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};

#[derive(Debug)]
pub struct Failed {
    child: ActorRef,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedFailed {
    child: SerializedActorRef,
    error: String,
}

impl CodecMessage for Failed {
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
                let serialized: SerializedFailed = decode_bytes(bytes)?;
                let child = provider.resolve_actor_ref(&serialized.child.path);
                let message = SystemDelegate::new(Failed { child, error: serialized.error });
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized = SerializedFailed {
            child: self.child.clone().into(),
            error: self.error.clone(),
        };
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

#[async_trait]
impl SystemMessage for Failed {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        todo!()
    }
}