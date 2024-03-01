use async_trait::async_trait;

use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::handoff_stopper::HandoffStopper;
use crate::shard_coordinator::shard_stopped::ShardStopped;

#[derive(Debug, EmptyCodec)]
pub(super) struct EntityTerminated(pub(super) ActorRef);

impl Terminated for EntityTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for EntityTerminated {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let entity = self.0;
        actor.remaining_entities.remove(&entity);
        if actor.remaining_entities.is_empty() {
            actor.replay_to.cast_ns(ShardStopped { shard: actor.shard.clone() });
            context.stop(context.myself());
        }
        Ok(())
    }
}