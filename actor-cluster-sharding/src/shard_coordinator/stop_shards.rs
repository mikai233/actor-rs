use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::ops::Not;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use itertools::Itertools;
use tracing::{info, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::ext::type_name_of;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::stop_shard_timeout::StopShardTimeout;
use crate::shard_region::ImShardId;

#[derive(Debug, EmptyCodec)]
pub(super) struct StopShards {
    pub(super) shards: HashSet<ImShardId>,
}

#[async_trait]
impl Message for StopShards {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard_ids = self.shards;
        if actor.state.regions.is_empty().not() && actor.preparing_for_shutdown.not() {
            let request_id = uuid::Uuid::new_v4();
            let (running_shards, already_stopped_shards): (Vec<_>, Vec<_>) = shard_ids
                .iter()
                .partition(|shard_id| { actor.state.shards.contains_key(*shard_id) });
            let sender = context.sender().into_result().context(type_name_of::<StopShards>())?;
            for shard_id in already_stopped_shards {
                sender.cast_ns(ShardStopped { shard: shard_id.clone().into() });
            }
            if running_shards.is_empty().not() {
                actor.waiting_for_shards_to_stop = running_shards
                    .iter()
                    .fold(actor.waiting_for_shards_to_stop.clone(), |mut acc, shard| {
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
                    });
                let shards_to_stop = running_shards
                    .into_iter()
                    .filter(|shard| {
                        !actor.rebalance_in_progress.contains_key(*shard)
                    })
                    .collect_vec();
                let shards_to_stop_str = shards_to_stop
                    .iter()
                    .map(|shard| shard.as_str()).join(", ");
                info!("{}: Explicitly stopping shards [{}] (request id [{}])", actor.type_name, shards_to_stop_str, request_id);
                let shard_per_region = shards_to_stop
                    .into_iter()
                    .flat_map(|shard_id| {
                        actor.state.shards.get(shard_id).map(|region| { (region.clone(), shard_id.clone()) })
                    })
                    .into_group_map();
                for (region, shards) in shard_per_region {
                    //TODO log error
                    actor.shutdown_shards(
                        context,
                        region.clone(),
                        shards.into_iter().collect(),
                    )?;
                }
                let timeout = StopShardTimeout(request_id);
                actor.timers.start_single_timer(
                    actor.settings.handoff_timeout,
                    timeout.into_dyn(),
                    context.myself().clone(),
                );
            }
        } else {
            let shard_ids_str = shard_ids
                .iter()
                .map(|shard| shard.as_str())
                .collect_vec().join(", ");
            warn!(
                "{}: Explicit stop shards of shards [{}] ignored (no known regions or shading shutting down)",
                actor.type_name,
                shard_ids_str,
            );
        }
        Ok(())
    }
}