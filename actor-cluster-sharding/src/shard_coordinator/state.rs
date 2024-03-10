use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use actor_core::actor_ref::ActorRef;

use crate::shard_region::ImShardId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub(super) shards: HashMap<ImShardId, ActorRef>,
    pub(super) regions: HashMap<ActorRef, HashSet<ImShardId>>,
    pub(super) region_proxies: HashSet<ActorRef>,
}

impl State {
    pub(super) fn update(&mut self) {
        todo!()
    }
}