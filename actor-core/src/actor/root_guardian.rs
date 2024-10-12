use tokio::sync::broadcast::Sender;
use tracing::debug;

use crate::{actor::context::Context, message::stop_child::StopChild};

use super::{receive::Receive, Actor};

#[derive(Debug)]
pub(crate) struct RootGuardian {
    termination_tx: Sender<()>,
}

impl RootGuardian {
    pub(crate) fn new(termination_tx: Sender<()>) -> Self {
        Self { termination_tx }
    }
}

impl Actor for RootGuardian {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        debug!("{} started", ctx.myself());
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        debug!("{} stopped", ctx.myself());
        let _ = self.termination_tx.send(());
        Ok(())
    }

    fn receive(&self) -> super::receive::Receive<Self> {
        Receive::new().handle::<StopChild>()
    }
}
