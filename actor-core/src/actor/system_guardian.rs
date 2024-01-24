use async_trait::async_trait;
use tracing::debug;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};

#[derive(Debug)]
pub(crate) struct SystemGuardian;

#[derive(EmptyCodec)]
pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

#[async_trait]
impl Message for StopChild {
    type A = SystemGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

#[async_trait]
impl Actor for SystemGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} started", context.myself());
        Ok(())
    }
}
