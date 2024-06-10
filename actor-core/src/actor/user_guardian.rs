use async_trait::async_trait;
use tracing::debug;

use crate::{Actor, DynMessage};
use crate::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct UserGuardian;


#[async_trait]
impl Actor for UserGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
