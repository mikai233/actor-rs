use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use actor_core::actor_ref::ActorRef;

use crate::shard_region::ShardId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub(super) shards: HashMap<ShardId, ActorRef>,
    pub(super) regions: HashMap<ActorRef, HashSet<ShardId>>,
    pub(super) regions_proxies: HashSet<ActorRef>,
}

impl State {
    pub(super) fn update(&mut self) {
        todo!()
    }
}