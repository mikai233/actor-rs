use std::ops::Not;

use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::CEmptyCodec;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, CEmptyCodec)]
pub(super) struct Retry;

#[async_trait]
impl Message for Retry {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.shard_buffers.is_empty().not() {
            actor.retry_count += 1;
        }
        if actor.coordinator.is_none() {
            actor.register(context)?;
        } else {
            actor.try_request_shard_buffer_homes(context);
        }
        actor.send_graceful_shutdown_to_coordinator_if_in_progress(context)?;
        actor.try_complete_graceful_shutdown_if_in_progress(context);
        Ok(())
    }
}