use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::{CodecMessage, DynMessage, SystemMessage};
use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::context::{ActorContext, Context};
use crate::decoder::MessageDecoder;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};
use crate::provider::{ActorRefProvider, TActorRefProvider};

#[derive(Debug)]
pub(crate) struct Unwatch {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedUnwatch {
    watchee: SerializedActorRef,
    watcher: SerializedActorRef,
}

impl CodecMessage for Unwatch {
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
                let serialized: SerializedUnwatch = decode_bytes(bytes)?;
                let watchee = provider.resolve_actor_ref(&serialized.watchee.path);
                let watcher = provider.resolve_actor_ref(&serialized.watcher.path);
                let message = SystemDelegate::new(Unwatch { watchee, watcher });
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized = SerializedUnwatch {
            watchee: self.watchee.clone().into(),
            watcher: self.watcher.clone().into(),
        };
        Some(encode_bytes(&serialized))
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        None
    }
}

#[async_trait]
impl SystemMessage for Unwatch {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        let Unwatch { watchee, watcher } = *self;
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if context.watched_by.remove(&watcher) {
                debug!("{} no longer watched by {}", context.myself, watcher);
            }
        } else if !watchee_self && watcher_self {
            context.unwatch(&watchee);
        } else {
            error!("illegal Unwatch({},{}) for {}", watchee, watcher, context.myself);
        }
        Ok(())
    }
}