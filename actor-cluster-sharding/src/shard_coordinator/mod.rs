use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::sync::Arc;

use async_trait::async_trait;
use itertools::Itertools;
use tracing::debug;

use actor_cluster::cluster::Cluster;
use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::state::State;
use crate::shard_region::shard_homes::ShardHomes;
use crate::shard_region::ShardId;

pub(crate) mod get_shard_home;
pub(crate) mod shard_stopped;
pub(crate) mod terminate_coordinator;
pub(crate) mod register;
pub(crate) mod register_proxy;
pub(crate) mod shard_started;
pub(crate) mod begin_handoff_ack;
mod rebalance_worker;
mod rebalance_done;
mod state;

#[derive(Debug)]
pub struct ShardCoordinator {
    type_name: String,
    settings: Arc<ClusterShardingSettings>,
    allocation_strategy: Box<dyn ShardAllocationStrategy>,
    cluster: Cluster,
    all_regions_registered: bool,
    state: State,
    preparing_for_shutdown: bool,
    rebalance_in_progress: HashMap<String, HashSet<ActorRef>>,
    rebalance_workers: HashSet<ActorRef>,
    un_acked_host_shards: HashMap<String, ()>,
    graceful_shutdown_in_progress: HashSet<ActorRef>,
    waiting_for_local_region_to_terminate: bool,
    alive_regions: HashSet<ActorRef>,
    region_termination_in_progress: HashSet<ActorRef>,
    waiting_for_shards_to_stop: HashMap<ShardId, HashSet<ActorRef>>,//TODO
}

impl ShardCoordinator {
    pub(crate) fn new(
        context: &mut ActorContext,
        type_name: String,
        settings: Arc<ClusterShardingSettings>,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
    ) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self {
            type_name,
            settings,
            allocation_strategy,
            cluster,
            all_regions_registered: false,
            state: Default::default(),
            preparing_for_shutdown: false,
            rebalance_in_progress: Default::default(),
            rebalance_workers: Default::default(),
            un_acked_host_shards: Default::default(),
            graceful_shutdown_in_progress: Default::default(),
            waiting_for_local_region_to_terminate: false,
            alive_regions: Default::default(),
            region_termination_in_progress: Default::default(),
            waiting_for_shards_to_stop: Default::default(),
        }
    }
}

#[async_trait]
impl Actor for ShardCoordinator {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}

impl ShardCoordinator {
    fn clear_rebalance_in_progress(&mut self, context: &mut ActorContext, shard: ShardId) {
        if let Some(pending_get_shard_home) = self.rebalance_in_progress.remove(&shard) {
            let msg = GetShardHome { shard };
            let myself = context.myself();
            for get_shard_home_sender in pending_get_shard_home {
                myself.cast(msg.clone(), Some(get_shard_home_sender));
            }
        }
    }

    fn is_member(&self, region: &ActorRef) -> bool {
        todo!()
    }

    fn inform_about_current_shards(&self, region: &ActorRef) {
        const BATCH_SIZE: usize = 500;
        if self.state.shards.is_empty().not() {
            debug!(
                "{}: Informing [{}] about (up to) [{}] shards in batches of [{}]",
                self.type_name,
                region, 
                self.state.shards.len(), 
                BATCH_SIZE,
            );
            self.state.regions.iter()
                .flat_map(|(region_ref, shards)| {
                    shards.iter()
                        .filter(|shard| { self.rebalance_in_progress.contains_key(*shard) })
                        .map(|shard| { (region_ref.clone(), shard) })
                }).chunks(BATCH_SIZE)
                .into_iter()
                .take(10)
                .for_each(|regions| {
                    let shards_sub_map = regions.into_iter()
                        .fold(HashMap::<ActorRef, Vec<ShardId>>::new(), |mut map, (region_ref, shard_id)| {
                            match map.entry(region_ref) {
                                Entry::Occupied(mut o) => {
                                    o.get_mut().push(shard_id.clone());
                                }
                                Entry::Vacant(v) => {
                                    v.insert(vec![shard_id.clone()]);
                                }
                            }
                            map
                        });
                    region.cast_ns(ShardHomes { homes: shards_sub_map });
                });
        }
    }
}