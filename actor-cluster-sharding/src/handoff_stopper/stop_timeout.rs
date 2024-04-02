use async_trait::async_trait;
use tracing::warn;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::handoff_stopper::HandoffStopper;

#[derive(Debug, EmptyCodec)]
pub(super) struct StopTimeout;

#[async_trait]
impl Message for StopTimeout {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let type_name = &actor.type_name;
        let stop_msg = actor.stop_message.name();
        let shard = &actor.shard;
        let timeout = &actor.entity_handoff_timeout;
        let remaining_size = actor.remaining_entities.len();
        warn!("{type_name}: handoff stop message [{stop_msg}] is not handled by some of the entities in shard [{shard}] after [{timeout:?}], stopping the remaining [{remaining_size}] entities");
        if let Some(key) = actor.stop_timeout_warning_key.take() {
            key.cancel();
        }
        for entity in &actor.remaining_entities {
            context.stop(entity);
        }
        Ok(())
    }
}