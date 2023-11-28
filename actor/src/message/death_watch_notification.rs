use std::any::Any;

use async_trait::async_trait;

use crate::{CodecMessage, DynMessage, SystemMessage};
use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::context::ActorContext;
use crate::decoder::MessageDecoder;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};
use crate::provider::{ActorRefProvider, TActorRefProvider};

pub(crate) struct DeathWatchNotification(pub(crate) ActorRef);

impl CodecMessage for DeathWatchNotification {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        #[derive(Clone)]
        struct D;
        impl MessageDecoder for D {
            fn decode(&self, provider: &ActorRefProvider, bytes: &[u8]) -> anyhow::Result<DynMessage> {
                let serialized: SerializedActorRef = decode_bytes(bytes)?;
                let actor_ref = provider.resolve_actor_ref(&serialized.path);
                let message = SystemDelegate::new(DeathWatchNotification(actor_ref));
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized: SerializedActorRef = self.0.clone().into();
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        context.watched_actor_terminated(self.0);
        Ok(())
    }
}