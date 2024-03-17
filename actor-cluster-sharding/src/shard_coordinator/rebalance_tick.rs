use std::ops::Not;

use async_trait::async_trait;
use itertools::Itertools;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::Message;
use actor_derive::CEmptyCodec;

use crate::shard_coordinator::rebalance_result::RebalanceResult;
use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Clone, CEmptyCodec)]
pub(super) struct RebalanceTick;

#[async_trait]
impl Message for RebalanceTick {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.state.regions.is_empty().not() && actor.preparing_for_shutdown.not() {
            let strategy = actor.allocation_strategy.clone();
            let regions = actor.state.regions
                .clone()
                .into_iter()
                .map(|(region, shards)| {
                    (region, shards.iter().map(|shard| { shard.clone() }).collect_vec())
                })
                .collect();
            let rebalance_in_progress = actor.rebalance_in_progress
                .keys()
                .map(|shard| { shard.clone() })
                .collect();
            let myself = context.myself().clone();
            let type_name = actor.type_name.clone();
            context.spawn_fut(async move {
                match strategy.rebalance(regions, rebalance_in_progress).await {
                    Ok(shards) => {
                        myself.cast_ns(RebalanceResult { shards });
                    }
                    Err(error) => {
                        debug!("{}: Rebalance error: {:#?}", type_name, error);
                        myself.cast_ns(RebalanceResult { shards: Default::default() });
                    }
                }
            });
        }
        Ok(())
    }
}