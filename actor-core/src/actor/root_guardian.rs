use async_trait::async_trait;
use tracing::debug;

use crate::Actor;
use crate::actor::context::{ActorContext, Context};

#[derive(Default)]
pub(crate) struct RootGuardian;

#[async_trait]
impl Actor for RootGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} stopped", context.myself());
        Ok(())
    }
}