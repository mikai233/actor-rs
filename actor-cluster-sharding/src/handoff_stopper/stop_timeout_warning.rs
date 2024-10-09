use async_trait::async_trait;
use tracing::warn;

use actor_core::actor::context::Context;
use actor_core::actor::coordinated_shutdown::CoordinatedShutdown;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::handoff_stopper::{HandoffStopper, STOP_TIMEOUT_WARNING_AFTER};

#[derive(Debug, EmptyCodec)]
pub(super) struct StopTimeoutWarning;

#[async_trait]
impl Message for StopTimeoutWarning {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let is_terminating = CoordinatedShutdown::get(context.system()).is_terminating();
        let type_name = &actor.type_name;
        let remaining_size = actor.remaining_entities.len();
        let shard = &actor.shard;
        let timeout = &STOP_TIMEOUT_WARNING_AFTER;
        let stop_msg = actor.stop_message.name();
        if is_terminating {
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled?");
        } else {
            let remaining_timeout = actor.entity_handoff_timeout - *timeout;
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled? \
            Waiting additional [{remaining_timeout:?}] before stopping the remaining entities.");
        }
        Ok(())
    }
}