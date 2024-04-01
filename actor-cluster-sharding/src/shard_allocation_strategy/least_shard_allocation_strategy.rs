use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_state::ClusterState;
use actor_cluster::member::{Member, MemberStatus};
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
        _requester: ActorRef,
        _shard_id: ImShardId,
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
        let limit = |number_of_shards: usize| {
            max(1, min((self.relative_limit * number_of_shards as f64) as usize, self.absolute_limit as usize))
        };
        let rebalance_phase1 = |number_of_shards: usize, optimal_per_region: usize, sorted_entries: Vec<RegionEntry>| {
            let mut selected = vec![];
            for RegionEntry { shard_ids, .. } in sorted_entries {
                if shard_ids.len() > optimal_per_region {
                    let len = shard_ids.len();
                    selected.extend(shard_ids.into_iter().take(len - optimal_per_region));
                }
            }
            selected.into_iter().take(limit(number_of_shards))
        };
        let rebalance_phase2 = |number_of_shards: usize, optimal_per_region: usize, sorted_entries: Vec<RegionEntry>| {
            let count_below_optimal: usize = sorted_entries.iter().map(|e| {
                optimal_per_region.checked_sub(1).and_then(|x| { x.checked_sub(e.shard_ids.len()) }).unwrap_or(0)
            }).sum();
            if count_below_optimal == 0 {
                HashSet::new()
            } else {
                let mut selected = vec![];
                for RegionEntry { shard_ids, .. } in sorted_entries {
                    if shard_ids.len() >= optimal_per_region {
                        selected.extend(shard_ids.into_iter().take(1));
                    }
                }
                selected.into_iter().take(min(count_below_optimal, limit(number_of_shards))).collect()
            }
        };
        if rebalance_in_progress.is_empty() {
            let mut sorted_region_entries = self.region_entries_for(current_shard_allocations);
            sorted_region_entries.sort_by(|x, y| {
                let RegionEntry { member: member_x, shard_ids: shard_ids_x, .. } = x;
                let RegionEntry { member: member_y, shard_ids: shard_ids_y, .. } = y;
                if member_x.status != member_y.status {
                    let x_is_leaving = matches!(member_x.status, MemberStatus::Leaving | MemberStatus::Removed);
                    let y_is_leaving = matches!(member_y.status, MemberStatus::Leaving | MemberStatus::Removed);
                    x_is_leaving.cmp(&y_is_leaving)
                } else {
                    shard_ids_x.len().cmp(&shard_ids_y.len())
                }
            });
            let is_a_good_time_to_rebalance = self.cluster.members().values().all(|m| { matches!(m.status, MemberStatus::Up) });
            if !is_a_good_time_to_rebalance {
                Ok(HashSet::new())
            } else {
                let number_of_shards: usize = sorted_region_entries.iter().map(|e| { e.shard_ids.len() }).sum();
                let number_of_regions = sorted_region_entries.len();
                if number_of_regions == 0 || number_of_shards == 0 {
                    Ok(HashSet::new())
                } else {
                    let mut optimal_per_region = number_of_shards / number_of_regions;
                    if number_of_shards % number_of_regions != 0 {
                        optimal_per_region += 1;
                    }
                    let result1 = rebalance_phase1(number_of_shards, optimal_per_region, sorted_region_entries.clone()).collect::<HashSet<_>>();
                    if result1.is_empty().not() {
                        Ok(result1)
                    } else {
                        let result2 = rebalance_phase2(number_of_shards, optimal_per_region, sorted_region_entries);
                        Ok(result2)
                    }
                }
            }
        } else {
            Ok(HashSet::new())
        }
    }
}