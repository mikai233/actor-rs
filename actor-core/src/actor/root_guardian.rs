use tokio::sync::broadcast::Sender;
use tracing::debug;

use crate::actor::context::{Context, ActorContext};

use super::Actor;

pub(crate) struct RootGuardian {
    termination_tx: Sender<()>,
}

impl RootGuardian {
    pub(crate) fn new(termination_tx: Sender<()>) -> Self {
        Self { termination_tx }
    }
}

impl Actor for RootGuardian {
    async fn started(&mut self, context: &mut Context<Self>) -> anyhow::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut Context<Self>) -> anyhow::Result<()> {
        debug!("{} stopped", context.myself());
        let _ = self.termination_tx.send(());
        Ok(())
    }

    fn receive(&self) -> super::receive::Receive<Self> {
        todo!()
    }
}
