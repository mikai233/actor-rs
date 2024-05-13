use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct CoordinatorTerminated(pub(super) Terminated);

impl CoordinatorTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for CoordinatorTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.coordinator.as_ref().is_some_and(|coordinator| coordinator == &*self.0) {
            actor.coordinator = None;
            actor.start_registration(context)?;
        }
        Ok(())
    }
}