use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::message::stop_child::StopChild;
use tracing::debug;

use super::context::Context;

#[derive(Debug)]
pub(crate) struct SystemGuardian;

impl Actor for SystemGuardian {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        debug!("{} started", ctx.myself());
        Ok(())
    }


    fn receive(&self) -> Receive<Self> {
        Receive::new().handle::<StopChild>()
    }
}
