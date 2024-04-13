use std::fmt::{Display, Formatter};

use ahash::{HashMap, HashSet, HashSetExt};
use bincode::{Decode, Encode};
use itertools::Itertools;

use actor_core::actor_ref::ActorRef;

use crate::shard_coordinator::state_update::ShardState;
use crate::shard_region::ImShardId;

#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) shards: HashMap<ImShardId, ActorRef>,
    pub(super) regions: HashMap<ActorRef, HashSet<ImShardId>>,
    pub(super) region_proxies: HashSet<ActorRef>,
}

impl State {
    pub(super) fn updated(&mut self, update: ShardState) {
        match update {
            ShardState::ShardHomeDeallocated { shard } => {
                debug_assert!(self.shards.contains_key(&shard), "Shard {} not allocated: {}", shard, self);
                self.shards.remove(&shard);
                for (_, shards) in &mut self.regions {
                    shards.remove(&shard);
                }
            }
            ShardState::ShardRegionProxyTerminated { region_proxy } => {
                debug_assert!(self.region_proxies.contains(&region_proxy), "Terminated region proxy {} not registered: {:?}", region_proxy, self);
                self.region_proxies.remove(&region_proxy);
            }
            ShardState::ShardCoordinatorInitialized => {}
            ShardState::ShardRegionTerminated { region } => {
                debug_assert!(self.regions.contains_key(&region), "Terminated region {} not registered: {:?}", region, self);
                if let Some(shards) = self.regions.remove(&region) {
                    for shard_id in shards {
                        self.shards.remove(&shard_id);
                    }
                }
            }
            ShardState::ShardRegionProxyRegistered { region_proxy } => {
                debug_assert!(!self.region_proxies.contains(&region_proxy), "Region proxy {} already registered: {:?}", region_proxy, self);
                self.region_proxies.insert(region_proxy);
            }
            ShardState::ShardHomeAllocated { shard, region } => {
                debug_assert!(self.regions.contains_key(&region), "Region {} not registered: {:?}", region, self);
                debug_assert!(!self.shards.contains_key(&shard), "Shard [{}] already allocated: {:?}", shard, self);
                if let Some(shards) = self.regions.get_mut(&region) {
                    shards.insert(shard.clone());
                }
                self.shards.insert(shard, region);
            }
            ShardState::ShardRegionRegistered { region } => {
                debug_assert!(!self.regions.contains_key(&region), "Region {} already registered: {:?}", region, self);
                self.regions.insert(region, HashSet::new());
            }
        }
    }

    pub(super) fn bin_state(&self) -> BinState {
        let shards: HashMap<String, ActorRef> = self.shards.iter()
            .map(|(shard_id, region)| { (shard_id.clone().into(), region.clone()) })
            .collect();
        let regions: HashMap<ActorRef, HashSet<String>> = self.regions.iter()
            .map(|(region, shards)| {
                (region.clone(), shards.iter().map(|shard| { shard.clone().into() }).collect())
            })
            .collect();
        let region_proxies = self.region_proxies.clone();
        BinState {
            shards,
            regions,
            region_proxies,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let shards = self.shards.iter().map(|(shard_id, region)| {
            format!("<{},{}>", shard_id, region)
        }).join(", ");
        let regions = self.regions.iter().map(|(region, shards)| {
            format!("<{},{}>", region, shards.iter().join(", "))
        }).join(", ");
        let region_proxies = self.region_proxies.iter().map(|proxy| { proxy.to_string() }).join(", ");
        write!(f, "State {{ shards: {}, regions: {}, region_proxies: {} }}", shards, regions, region_proxies)
    }
}

#[derive(Debug, Encode, Decode)]
pub(super) struct BinState {
    pub(super) shards: HashMap<String, ActorRef>,
    pub(super) regions: HashMap<ActorRef, HashSet<String>>,
    pub(super) region_proxies: HashSet<ActorRef>,
}

impl From<BinState> for State {
    fn from(value: BinState) -> Self {
        let shards: HashMap<ImShardId, ActorRef> = value.shards.into_iter()
            .map(|(shard_id, region)| { (shard_id.into(), region) })
            .collect();
        let regions: HashMap<ActorRef, HashSet<ImShardId>> = value.regions.into_iter()
            .map(|(region, shards)| {
                (region, shards.into_iter().map(|shard| { shard.into() }).collect())
            })
            .collect();
        Self {
            shards,
            regions,
            region_proxies: value.region_proxies,
        }
    }
}