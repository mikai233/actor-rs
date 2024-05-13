use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::shard_coordinator::coordinator_state::CoordinatorState;
use crate::shard_coordinator::shard_region_terminated::ShardRegionTerminated;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_region::register_ack::RegisterAck;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct Register {
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for Register {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let region = self.shard_region;
        if matches!(actor.coordinator_state, CoordinatorState::WaitingForStateInitialized) {
            debug!("{}: Ignoring registration from region [{}] while initializing", actor.type_name, region);
            return Ok(());
        }
        if actor.is_member(context, &region) {
            debug!("{}: ShardRegion registered: [{}]", actor.type_name, region);
            actor.alive_regions.insert(region.clone());
            if actor.state.regions.contains_key(&region) {
                region.cast_ns(RegisterAck { coordinator: context.myself().clone() });
                actor.inform_about_current_shards(&region);
            } else {
                actor.graceful_shutdown_in_progress.remove(&region);
                actor.inform_about_current_shards(&region);
                actor.update_state(context, ShardState::ShardRegionRegistered { region: region.clone() }).await;
                if context.is_watching(&region).not() {
                    context.watch(region.clone(), ShardRegionTerminated::new)?;
                }
                region.cast_ns(RegisterAck { coordinator: context.myself().clone() });
            }
        } else {
            debug!(
                "{}: ShardRegion [{}] was not registered since the coordinator currently does not know about a node of that region",
                actor.type_name,
                region,
            );
        }
        Ok(())
    }
}