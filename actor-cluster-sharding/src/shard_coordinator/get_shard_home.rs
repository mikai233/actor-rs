use std::any::type_name;
use std::ops::Not;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use ahash::HashMap;
use anyhow::anyhow;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::shard_coordinator::allocate_shard_result::AllocateShardResult;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::{ImShardId, ShardId};

#[derive(
    Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Message, MessageCodec,
)]
pub(crate) struct GetShardHome {
    pub(crate) shard: ShardId,
}

impl MessageHandler<ShardCoordinator> for GetShardHome {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut ShardCoordinator::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let sender = sender.ok_or(anyhow!("Sender is None"))?;
        let shard: ImShardId = message.shard.into();
        if !actor.handle_get_shard_home(ctx, sender.clone(), shard.clone()) {
            let active_regions: HashMap<_, _> = actor
                .state
                .regions
                .iter()
                .filter(|(region, _)| {
                    actor.graceful_shutdown_in_progress.contains(region).not()
                        && actor.region_termination_in_progress.contains(region).not()
                })
                .map(|(region, shards)| {
                    (
                        region.clone(),
                        shards.iter().map(|shard| shard.clone()).collect_vec(),
                    )
                })
                .collect();
            if active_regions.is_empty().not() {
                let strategy = actor.allocation_strategy.clone();
                let myself = ctx.myself().clone();
                let type_name = actor.type_name.clone();
                ctx.spawn_async("allocate_shard", async move {
                    match strategy
                        .allocate_shard(sender.clone(), shard.clone(), active_regions)
                        .await
                    {
                        Ok(region) => {
                            myself.cast_ns(AllocateShardResult {
                                shard,
                                shard_region: Some(region),
                                get_shard_home_sender: sender,
                            });
                        }
                        Err(error) => {
                            error!(
                                "{}: Shard [{}] allocation failed. {:?}",
                                type_name, shard, error
                            );
                            myself.cast_ns(AllocateShardResult {
                                shard,
                                shard_region: None,
                                get_shard_home_sender: sender,
                            });
                        }
                    };
                })?;
            }
        }
        Ok(Behavior::same())
    }
}
