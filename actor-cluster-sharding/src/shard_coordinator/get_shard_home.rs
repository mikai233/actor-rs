use std::any::type_name;
use std::ops::Not;

use ahash::HashMap;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use eyre::Context as _;
use itertools::Itertools;
use tracing::error;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::allocate_shard_result::AllocateShardResult;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::{ImShardId, ShardId};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, MessageCodec)]
pub(crate) struct GetShardHome {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for GetShardHome {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let sender = context.sender().into_result().context(type_name::<GetShardHome>())?.clone();
        let shard: ImShardId = self.shard.into();
        if !actor.handle_get_shard_home(context, sender.clone(), shard.clone()) {
            let active_regions: HashMap<_, _> = actor.state.regions.iter().filter(|(region, _)| {
                actor.graceful_shutdown_in_progress.contains(region).not() && actor.region_termination_in_progress.contains(region).not()
            }).map(|(region, shards)| {
                (region.clone(), shards.iter().map(|shard| { shard.clone() }).collect_vec())
            }).collect();
            if active_regions.is_empty().not() {
                let strategy = actor.allocation_strategy.clone();
                let myself = context.myself().clone();
                let type_name = actor.type_name.clone();
                context.spawn_fut(async move {
                    match strategy.allocate_shard(sender.clone(), shard.clone(), active_regions).await {
                        Ok(region) => {
                            myself.cast_ns(AllocateShardResult {
                                shard,
                                shard_region: Some(region),
                                get_shard_home_sender: sender,
                            });
                        }
                        Err(error) => {
                            error!("{}: Shard [{}] allocation failed. {:?}", type_name, shard, error);
                            myself.cast_ns(AllocateShardResult {
                                shard,
                                shard_region: None,
                                get_shard_home_sender: sender,
                            });
                        }
                    };
                });
            }
        }
        Ok(())
    }
}