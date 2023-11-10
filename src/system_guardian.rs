use async_trait::async_trait;
use crate::actor::context::{ActorContext, Context};
use crate::actor::{Actor, Message};
use crate::cell::envelope::UserEnvelope;
use tracing::debug;
use crate::actor_ref::ActorRef;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct SystemGuardian;

pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

#[async_trait(? Send)]
impl Message for StopChild {
    type T = SystemGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext<'_, Self::T>, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

impl Actor for SystemGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("SystemGuardian {} pre start", ctx.myself());
        Ok(())
    }
}
