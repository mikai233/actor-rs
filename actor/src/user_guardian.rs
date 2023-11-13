use std::any::Any;
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use futures::stream::FuturesUnordered;
use tracing::debug;

use crate::actor::context::{ActorContext, Context};
use crate::actor::{Actor, CodecMessage, Message};
use crate::actor_ref::ActorRef;
use crate::cell::envelope::UserEnvelope;
use crate::decoder::MessageDecoder;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct UserGuardian;

#[derive(Debug)]
pub(crate) enum UserGuardianMessage {
    StopChild {
        child: ActorRef,
    }
}

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

#[async_trait]
impl Message for StopChild {
    type T = UserGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

impl Actor for UserGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("UserGuardian {} pre start", context.myself());
        Ok(())
    }
}
