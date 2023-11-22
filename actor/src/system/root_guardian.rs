use tracing::debug;

use crate::Actor;
use crate::context::{ActorContext, Context};

#[derive(Debug)]
pub struct RootGuardian;

impl Actor for RootGuardian {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("{} pre start", context.myself());
        Ok(())
    }
}
