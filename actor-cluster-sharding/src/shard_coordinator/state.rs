use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use actor_core::actor_ref::ActorRef;

use crate::shard_coordinator::state_update::StateUpdate;
use crate::shard_region::ImShardId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub(super) shards: HashMap<ImShardId, ActorRef>,
    pub(super) regions: HashMap<ActorRef, HashSet<ImShardId>>,
    pub(super) region_proxies: HashSet<ActorRef>,
}

impl State {
    pub(super) fn updated(&mut self, update: StateUpdate) {
        match update {
            StateUpdate::ShardHomeDeallocated { shard } => {}
            StateUpdate::ShardRegionProxyTerminated { region_proxy } => {
                debug_assert!(self.region_proxies.contains(&region_proxy), "Terminated region proxy {} not registered: {:?}", region_proxy, self);
                self.region_proxies.remove(&region_proxy);
            }
            StateUpdate::ShardCoordinatorInitialized => {}
            StateUpdate::ShardRegionTerminated { region } => {
                debug_assert!(self.regions.contains_key(&region), "Terminated region {} not registered: {:?}", region, self);
                if let Some(shards) = self.regions.remove(&region) {
                    for shard_id in shards {
                        self.shards.remove(&shard_id);
                    }
                }
            }
            StateUpdate::ShardRegionProxyRegistered { region_proxy } => {
                debug_assert!(!self.region_proxies.contains(&region_proxy), "Region proxy {} already registered: {:?}", region_proxy, self);
                self.region_proxies.insert(region_proxy);
            }
            StateUpdate::ShardHomeAllocated { shard, region } => {
                debug_assert!(self.regions.contains_key(&region), "Region {} not registered: {:?}", region, self);
                debug_assert!(!self.shards.contains_key(&shard), "Shard [{}] already allocated: {:?}", shard, self);
                if let Some(shards) = self.regions.get_mut(&region) {
                    shards.insert(shard.clone());
                }
                self.shards.insert(shard, region);
            }
            StateUpdate::ShardRegionRegistered { region } => {
                debug_assert!(!self.regions.contains_key(&region), "Region {} already registered: {:?}", region, self);
                self.regions.insert(region, HashSet::new());
            }
        }
    }
}