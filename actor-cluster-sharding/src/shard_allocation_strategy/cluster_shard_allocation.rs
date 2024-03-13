use std::collections::HashMap;

use itertools::Itertools;

use actor_cluster::cluster_state::ClusterState;
use actor_cluster::member::Member;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRef;

use crate::shard_region::ImShardId;

#[derive(Debug)]
pub struct RegionEntry {
    pub region: ActorRef,
    pub member: Member,
    pub shard_ids: Vec<ImShardId>,
}

pub type AllocationMap = HashMap<ActorRef, Vec<ImShardId>>;

pub trait ClusterShardAllocation {
    fn cluster_state(&self) -> &ClusterState;

    fn self_member(&self) -> Member;

    fn region_entries_for(&self, current_shard_allocations: AllocationMap) -> Vec<RegionEntry> {
        let address_to_member: HashMap<_, _> = self.cluster_state()
            .members
            .read()
            .unwrap()
            .iter().map(|(addr, member)| {
            (addr.address.clone(), member.clone())
        }).collect();
        current_shard_allocations.into_iter().flat_map(|(region, shard_ids)| {
            let region_address = region.path().address();
            let member_for_region = address_to_member.get(region_address);
            member_for_region.map(|member| {
                RegionEntry {
                    region,
                    member: member.clone(),
                    shard_ids,
                }
            })
        }).collect_vec()
    }
}