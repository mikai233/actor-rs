use std::any::type_name;
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::ops::Not;

use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::stop_shard_timeout::StopShardTimeout;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use ahash::{HashSet, HashSetExt};
use anyhow::anyhow;
use itertools::Itertools;
use tracing::{info, warn};

#[derive(Debug, Message)]
pub(super) struct StopShards {
    pub(super) shards: HashSet<ImShardId>,
}

impl Display for StopShards {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StopShards {{ shards: {} }}",
            self.shards.iter().join(", ")
        )
    }
}

impl MessageHandler<ShardCoordinator> for StopShards {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut ShardCoordinator::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let shard_ids = message.shards;
        if actor.state.regions.is_empty().not() && actor.preparing_for_shutdown.not() {
            let request_id = uuid::Uuid::new_v4();
            let (running_shards, already_stopped_shards): (Vec<_>, Vec<_>) = shard_ids
                .iter()
                .partition(|shard_id| actor.state.shards.contains_key(*shard_id));
            let sender = sender.ok_or(anyhow!("Sender is None"))?;
            for shard_id in already_stopped_shards {
                sender.cast_ns(ShardStopped {
                    shard: shard_id.clone().into(),
                });
            }
            if running_shards.is_empty().not() {
                actor.waiting_for_shards_to_stop = running_shards.iter().fold(
                    actor.waiting_for_shards_to_stop.clone(),
                    |mut acc, shard| {
                        match acc.entry((*shard).clone()) {
                            Entry::Occupied(mut o) => {
                                o.get_mut().insert((sender.clone(), request_id));
                            }
                            Entry::Vacant(v) => {
                                let mut shards = HashSet::new();
                                shards.insert((sender.clone(), request_id));
                                v.insert(shards);
                            }
                        }
                        acc
                    },
                );
                let shards_to_stop = running_shards
                    .into_iter()
                    .filter(|shard| !actor.rebalance_in_progress.contains_key(*shard))
                    .collect_vec();
                let shards_to_stop_str =
                    shards_to_stop.iter().map(|shard| shard.as_str()).join(", ");
                info!(
                    "{}: Explicitly stopping shards [{}] (request id [{}])",
                    actor.type_name, shards_to_stop_str, request_id
                );
                let shard_per_region = shards_to_stop
                    .into_iter()
                    .flat_map(|shard_id| {
                        actor
                            .state
                            .shards
                            .get(shard_id)
                            .map(|region| (region.clone(), shard_id.clone()))
                    })
                    .into_group_map();
                for (region, shards) in shard_per_region {
                    let shutdown_result =
                        actor.shutdown_shards(ctx, region.clone(), shards.into_iter().collect());
                    if let Some(error) = shutdown_result.err() {
                        warn!(
                            "{}: shutdown shards of region {} error {:?}",
                            actor.type_name, region, error
                        );
                    }
                }
                actor.timers.start_single_timer(
                    actor.settings.handoff_timeout,
                    StopShardTimeout(request_id),
                    ctx.myself().clone(),
                );
            }
        } else {
            warn!(
                "{}: Explicit stop shards of shards [{}] ignored (no known regions or shading shutting down)",
                actor.type_name,
                shard_ids.iter().join(", ")
            );
        }
        Ok(Behavior::same())
    }
}
