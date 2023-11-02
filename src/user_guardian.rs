use tracing::debug;

use crate::actor::context::{ActorContext, Context};
use crate::actor::Actor;
use crate::cell::envelope::UserEnvelope;

#[derive(Debug)]
pub(crate) struct UserGuardian;

impl Actor for UserGuardian {
    type M = ();
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("UserGuardian {} pre start", ctx.myself());
        Ok(())
    }

    fn on_recv(
        &self,
        ctx: &mut ActorContext<Self>,
        state: &mut Self::S,
        message: UserEnvelope<Self::M>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
