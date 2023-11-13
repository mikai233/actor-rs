use tracing::debug;

use crate::actor::Actor;
use crate::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct RootGuardian;

impl Actor for RootGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("RootGuardian {} pre start", context.myself());
        Ok(())
    }
}
