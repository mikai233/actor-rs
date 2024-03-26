use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub(super) struct HandoffStopperTerminated(pub(super) Terminated);

impl HandoffStopperTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for HandoffStopperTerminated {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.handoff_stopper.as_ref().is_some_and(|a| a == &*self.0) {
            context.stop(context.myself());
        }
        Ok(())
    }
}