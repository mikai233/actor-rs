use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::{Actor, CodecMessage, DynMessage, SystemMessage};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::system::SystemDelegate;
use crate::ext::{decode_bytes, encode_bytes};

#[derive(Debug)]
pub struct Watch {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedWatch {
    watchee: SerializedActorRef,
    watcher: SerializedActorRef,
}

impl CodecMessage for Watch {
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
                let serialized: SerializedWatch = decode_bytes(bytes)?;
                let watchee = provider.resolve_actor_ref(&serialized.watchee.path);
                let watcher = provider.resolve_actor_ref(&serialized.watcher.path);
                let message = SystemDelegate::new(Watch { watchee, watcher });
                Ok(message.into())
            }
        }
        Some(Box::new(D))
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let serialized = SerializedWatch {
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
impl SystemMessage for Watch {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Watch { watchee, watcher } = *self;
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if !context.watched_by.contains(&watcher) {
                debug!("{} is watched by {}", context.myself, watcher);
                context.watched_by.insert(watcher);
            } else {
                debug!("watcher {} already added for {}", watcher, context.myself);
            }
        } else {
            error!("illegal Watch({},{}) for {}", watchee, watcher, context.myself);
        }
        Ok(())
    }
}