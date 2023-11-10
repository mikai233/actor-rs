use async_trait::async_trait;
use tracing::debug;

use crate::actor::context::{ActorContext, Context};
use crate::actor::{Actor, Message};
use crate::actor_ref::ActorRef;
use crate::cell::envelope::UserEnvelope;
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

#[async_trait(? Send)]
impl Message for StopChild {
    type T = UserGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext<'_, Self::T>, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

impl Actor for UserGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("UserGuardian {} pre start", ctx.myself());
        Ok(())
    }
}
