use serde::{Deserialize, Serialize};
use std::ops::Not;
use tracing::debug;

use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};

#[derive(Debug, Clone, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[display("GracefulShutdownReq {{ shard_region: {shard_region} }}")]
pub(crate) struct GracefulShutdownReq {
    pub(crate) shard_region: ActorRef,
}

impl MessageHandler<ShardCoordinator> for GracefulShutdownReq {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut ShardCoordinator::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let region = message.shard_region;
        if actor.graceful_shutdown_in_progress.contains(&region).not() {
            match actor.state.regions.get(&region) {
                None => {
                    debug!(
                        "{}: Unknown region requested graceful shutdown [{}]",
                        actor.type_name, region
                    );
                }
                Some(shards) => {
                    debug!(
                        "{}: Graceful shutdown of region [{}] with [{}] shards",
                        actor.type_name,
                        region,
                        shards.len()
                    );
                    actor.graceful_shutdown_in_progress.insert(region.clone());
                    actor.shutdown_shards(ctx, region, shards.clone())?;
                }
            }
        }
        Ok(Behavior::same())
    }
}
