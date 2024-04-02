use async_trait::async_trait;
use tracing::debug;

use crate::Actor;
use crate::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct SystemGuardian;

#[async_trait]
impl Actor for SystemGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }
}
