use std::ops::Not;

use crate::shard_coordinator::rebalance_result::RebalanceResult;
use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use itertools::Itertools;
use tracing::debug;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
pub(super) struct RebalanceTick;

impl MessageHandler<ShardCoordinator> for RebalanceTick {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        if actor.state.regions.is_empty().not() && actor.preparing_for_shutdown.not() {
            let strategy = actor.allocation_strategy.clone();
            let regions = actor
                .state
                .regions
                .clone()
                .into_iter()
                .map(|(region, shards)| {
                    (
                        region,
                        shards.iter().map(|shard| shard.clone()).collect_vec(),
                    )
                })
                .collect();
            let rebalance_in_progress = actor
                .rebalance_in_progress
                .keys()
                .map(|shard| shard.clone())
                .collect();
            let myself = ctx.myself().clone();
            let type_name = actor.type_name.clone();
            ctx.spawn_async("rebalance", async move {
                match strategy.rebalance(regions, rebalance_in_progress).await {
                    Ok(shards) => {
                        myself.cast_ns(RebalanceResult { shards });
                    }
                    Err(error) => {
                        debug!("{}: Rebalance error: {:?}", type_name, error);
                        myself.cast_ns(RebalanceResult {
                            shards: Default::default(),
                        });
                    }
                }
            })?;
        }
        Ok(Behavior::same())
    }
}
