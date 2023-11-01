use tracing::debug;
use crate::actor::Actor;
use crate::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct SystemGuardian;

impl Actor for SystemGuardian {
    type M = ();
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("SystemGuardian {} pre start",ctx.myself());
        Ok(())
    }

    fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
        Ok(())
    }
}