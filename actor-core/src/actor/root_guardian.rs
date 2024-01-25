use async_trait::async_trait;
use tokio::sync::broadcast::Sender;
use tracing::debug;

use crate::Actor;
use crate::actor::context::{ActorContext, Context};

pub(crate) struct RootGuardian {
    termination_tx: Sender<()>,
}

impl RootGuardian {
    pub(crate) fn new(termination_tx: Sender<()>) -> Self {
        Self {
            termination_tx,
        }
    }
}

#[async_trait]
impl Actor for RootGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} stopped", context.myself());
        let _ = self.termination_tx.send(());
        Ok(())
    }
}