use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_state::ClusterState;
use actor_cluster::member::Member;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor_ref::ActorRef;

use crate::shard_allocation_strategy::cluster_shard_allocation::{ClusterShardAllocation, RegionEntry};
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::ImShardId;

#[derive(Debug, Clone)]
pub struct LeastShardAllocationStrategy {
    pub absolute_limit: i32,
    pub relative_limit: f64,
    cluster: Cluster,
}

impl LeastShardAllocationStrategy {
    pub fn new(system: &ActorSystem, absolute_limit: i32, relative_limit: f64) -> Self {
        let cluster = Cluster::get(&system).clone();
        Self {
            absolute_limit,
            relative_limit,
            cluster,
        }
    }

    pub fn most_suitable_region(&self, region_entries: Vec<RegionEntry>) -> (ActorRef, Vec<ImShardId>) {
        let most_suitable_entry = region_entries.into_iter().min_by(|a, b| {
            a.shard_ids.len().cmp(&b.shard_ids.len())
        }).expect("region_entries is empty");
        (most_suitable_entry.region, most_suitable_entry.shard_ids)
    }
}

impl ClusterShardAllocation for LeastShardAllocationStrategy {
    fn cluster_state(&self) -> &ClusterState {
        self.cluster.state()
    }

    fn self_member(&self) -> Member {
        self.cluster.self_member().clone()
    }
}

#[async_trait]
impl ShardAllocationStrategy for LeastShardAllocationStrategy {
    async fn allocate_shard(
        &self,
        requester: ActorRef,
        shard_id: ImShardId,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
    ) -> anyhow::Result<ActorRef> {
        let mut region_entries: Vec<RegionEntry>;
        const MAX_LOOP: usize = 100;
        let mut current_loop = 0;
        loop {
            current_loop += 1;
            region_entries = self.region_entries_for(current_shard_allocations.clone());
            if region_entries.is_empty() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            } else {
                break;
            }
            if current_loop >= MAX_LOOP {
                return Err(anyhow!("allocate shard max loop {} reached", MAX_LOOP));
            }
        }
        let (region, _) = self.most_suitable_region(region_entries);
        Ok(region)
    }

    async fn rebalance(
        &self,
        current_shard_allocations: HashMap<ActorRef, Vec<ImShardId>>,
        rebalance_in_progress: Vec<ImShardId>,
    ) -> anyhow::Result<HashSet<ImShardId>> {
        todo!()
    }
}