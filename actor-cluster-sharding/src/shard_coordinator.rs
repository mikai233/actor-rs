use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use actor_cluster::cluster::Cluster;
use actor_core::Actor;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::ActorContext;

#[derive(Debug)]
pub struct ShardCoordinator {
    cluster: Cluster,
    rebalance_in_progress: HashMap<String, HashSet<ActorRef>>,
    rebalance_workers: HashSet<ActorRef>,
    un_acked_host_shards: HashMap<String, ()>,
    graceful_shutdown_in_progress: HashSet<ActorRef>,
    alive_regions:HashSet<ActorRef>,
}

#[async_trait]
impl Actor for ShardCoordinator {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}

