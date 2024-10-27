use std::fmt::{Debug, Display};

use actor_core::actor_ref::ActorRef;
use ahash::{HashMap, HashSet};
use dyn_clone::DynClone;
use futures::future::BoxFuture;

use crate::shard_region::ImShardId;

pub mod cluster_shard_allocation;
pub mod least_shard_allocation_strategy;

pub trait ShardAllocationStrategy: Send + Sync + Debug + DynClone + Display {
    fn allocate_shard(
        &self,
        requester: ActorRef,
        shard_id: ImShardId,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
    ) -> BoxFuture<'static, anyhow::Result<ActorRef>>;

    fn rebalance(
        &self,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
        rebalance_in_progress: Vec<ImShardId>,
    ) -> BoxFuture<'static, anyhow::Result<HashSet<ImShardId>>>;
}

dyn_clone::clone_trait_object!(ShardAllocationStrategy);