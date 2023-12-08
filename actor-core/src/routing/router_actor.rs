use std::any::Any;
use std::sync::Arc;

use crate::{Actor, CodecMessage, DynMessage, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::actor::serialized_ref::SerializedActorRef;
use crate::delegate::user::UserDelegate;
use crate::ext::{decode_bytes, encode_bytes};
use crate::message::terminated::WatchTerminated;

#[derive(Clone)]
pub struct RouterActor;

impl Actor for RouterActor {}

pub(crate) struct WatchRouteeTerminated(ActorRef);

impl Message for WatchRouteeTerminated {
    type A = RouterActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl CodecMessage for WatchRouteeTerminated {
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
            fn decode(&self, provider: &Arc<Box<dyn ActorRefProvider>>, bytes: &[u8]) -> anyhow::Result<DynMessage> {
                let serialized: SerializedActorRef = decode_bytes(bytes)?;
                let terminated = provider.resolve_actor_ref(&serialized.path);
                let message = UserDelegate::new(WatchRouteeTerminated(terminated));
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

impl WatchTerminated for WatchRouteeTerminated {
    fn watch_actor(&self) -> &ActorRef {
        &self.0
    }
}