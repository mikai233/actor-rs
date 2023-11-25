use async_trait::async_trait;
use tracing::debug;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::context::{ActorContext, Context};
use crate::provider::ActorRefFactory;
use crate::system::root_guardian::ChildGuardianStarted;

#[derive(Debug)]
pub(crate) struct SystemGuardian;

#[derive(EmptyCodec)]
pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

impl Message for StopChild {
    type T = SystemGuardian;

    fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}

#[async_trait]
impl Actor for SystemGuardian {
    type S = ();
    type A = ();

    async fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("{} pre start", context.myself());
        context.parent().unwrap().cast(ChildGuardianStarted { guardian: context.myself.clone() }, ActorRef::no_sender());
        Ok(())
    }
}
