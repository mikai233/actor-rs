use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::handoff_stopper::HandoffStopper;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;

#[derive(Debug, EmptyCodec)]
pub(super) struct EntityTerminated(pub(super) Terminated);

impl EntityTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for EntityTerminated {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let entity = self.0;
        actor.remaining_entities.remove(&entity);
        if actor.remaining_entities.is_empty() {
            actor.replay_to.cast_ns(ShardStopped { shard: actor.shard.clone().into() });
            context.stop(context.myself());
        }
        Ok(())
    }
}