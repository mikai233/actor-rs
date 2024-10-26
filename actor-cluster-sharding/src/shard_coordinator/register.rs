use serde::{Deserialize, Serialize};
use std::ops::Not;
use tracing::debug;

use crate::shard_coordinator::coordinator_state::CoordinatorState;
use crate::shard_coordinator::shard_region_terminated::ShardRegionTerminated;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::register_ack::RegisterAck;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};

#[derive(Debug, Clone, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[display("Register {{ shard_region: {shard_region} }}")]
#[cloneable]
pub(crate) struct Register {
    pub(crate) shard_region: ActorRef,
}

impl MessageHandler<ShardCoordinator> for Register {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let region = message.shard_region;
        if matches!(
            actor.coordinator_state,
            CoordinatorState::WaitingForStateInitialized
        ) {
            debug!(
                "{}: Ignoring registration from region [{}] while initializing",
                actor.type_name, region
            );
            return Ok(Behavior::same());
        }
        if actor.is_member(ctx, &region) {
            debug!("{}: ShardRegion registered: [{}]", actor.type_name, region);
            actor.alive_regions.insert(region.clone());
            if actor.state.regions.contains_key(&region) {
                region.cast_ns(RegisterAck {
                    coordinator: ctx.myself().clone(),
                });
                actor.inform_about_current_shards(&region);
            } else {
                actor.graceful_shutdown_in_progress.remove(&region);
                actor.inform_about_current_shards(&region);
                actor
                    .update_state(
                        ctx,
                        ShardState::ShardRegionRegistered {
                            region: region.clone(),
                        },
                    )
                    .await;
                if ctx.is_watching(&region).not() {
                    ctx.watch(&region)?;
                    context.watch_with(region.clone(), ShardRegionTerminated::new)?;
                }
                region.cast_ns(RegisterAck {
                    coordinator: ctx.myself().clone(),
                });
            }
        } else {
            debug!(
                "{}: ShardRegion [{}] was not registered since the coordinator currently does not know about a node of that region",
                actor.type_name,
                region,
            );
        }
        Ok(Behavior::same())
    }
}
