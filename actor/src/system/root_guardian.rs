use async_trait::async_trait;
use tracing::debug;

use crate::Actor;
use crate::context::{ActorContext, Context};

#[derive(Debug)]
pub struct RootGuardian;

#[async_trait]
impl Actor for RootGuardian {
    type S = ();
    type A = ();

    async fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("{} pre start", context.myself());
        Ok(())
    }
}
