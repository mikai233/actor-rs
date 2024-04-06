use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use async_trait::async_trait;
use dyn_clone::DynClone;

use actor_core::actor_ref::ActorRef;

use crate::shard_region::ImShardId;

pub mod cluster_shard_allocation;
pub mod least_shard_allocation_strategy;

#[async_trait]
pub trait ShardAllocationStrategy: Send + Sync + Debug + DynClone {
    async fn allocate_shard(
        &self,
        requester: ActorRef,
        shard_id: ImShardId,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
    ) -> eyre::Result<ActorRef>;

    async fn rebalance(
        &self,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
        rebalance_in_progress: Vec<ImShardId>,
    ) -> eyre::Result<HashSet<ImShardId>>;
}

dyn_clone::clone_trait_object!(ShardAllocationStrategy);