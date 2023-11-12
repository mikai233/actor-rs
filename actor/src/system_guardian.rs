use std::any::Any;
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use futures::stream::FuturesUnordered;
use crate::actor::context::{ActorContext, Context};
use crate::actor::{Actor, CodecMessage, Message};
use crate::cell::envelope::UserEnvelope;
use tracing::debug;
use crate::actor_ref::ActorRef;
use crate::decoder::MessageDecoder;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct SystemGuardian;

pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

impl CodecMessage for StopChild {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

#[async_trait(? Send)]
impl Message for StopChild {
    type T = SystemGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

impl Actor for SystemGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("SystemGuardian {} pre start", context.myself());
        Ok(())
    }
}
