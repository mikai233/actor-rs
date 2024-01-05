use async_trait::async_trait;
use tracing::debug;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::actor_ref::{ActorRef, ActorRefExt};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::actor::root_guardian::ChildGuardianStarted;

#[derive(Debug)]
pub(crate) struct UserGuardian;

#[derive(EmptyCodec)]
pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

#[async_trait]
impl Message for StopChild {
    type A = UserGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

#[async_trait]
impl Actor for UserGuardian {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} pre start", context.myself());
        context.parent().unwrap().cast(ChildGuardianStarted { guardian: context.myself.clone() }, ActorRef::no_sender());
        Ok(())
    }
}
