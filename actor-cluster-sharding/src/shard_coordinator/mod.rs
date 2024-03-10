use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use itertools::Itertools;
use tracing::{debug, info};

use actor_cluster::cluster::Cluster;
use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_coordinator::state::State;
use crate::shard_region::{ImShardId, ShardId};
use crate::shard_region::shard_homes::ShardHomes;

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
pub(crate) mod graceful_shutdown_req;
mod rebalance_tick;
mod rebalance_result;
mod stop_shard_timeout;
mod stop_shards;

#[derive(Debug)]
pub struct ShardCoordinator {
    type_name: String,
    settings: Arc<ClusterShardingSettings>,
    allocation_strategy: Box<dyn ShardAllocationStrategy>,
    cluster: Cluster,
    all_regions_registered: bool,
    state: State,
    preparing_for_shutdown: bool,
    rebalance_in_progress: HashMap<ImShardId, HashSet<ActorRef>>,
    rebalance_workers: HashSet<ActorRef>,
    un_acked_host_shards: HashMap<ImShardId, ()>,
    graceful_shutdown_in_progress: HashSet<ActorRef>,
    waiting_for_local_region_to_terminate: bool,
    alive_regions: HashSet<ActorRef>,
    region_termination_in_progress: HashSet<ActorRef>,
    waiting_for_shards_to_stop: HashMap<ImShardId, HashSet<ActorRef>>,//TODO
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
        if let Some(pending_get_shard_home) = self.rebalance_in_progress.remove(shard.as_str()) {
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
                        .filter(|shard| { self.rebalance_in_progress.contains_key(shard.as_str()) })
                        .map(|shard| { (region_ref.clone(), shard) })
                }).chunks(BATCH_SIZE)
                .into_iter()
                .take(10)
                .for_each(|regions| {
                    let shards_sub_map = regions.into_iter()
                        .fold(HashMap::<ActorRef, Vec<ShardId>>::new(), |mut map, (region_ref, shard_id)| {
                            match map.entry(region_ref) {
                                Entry::Occupied(mut o) => {
                                    o.get_mut().push(shard_id.clone().into());
                                }
                                Entry::Vacant(v) => {
                                    v.insert(vec![shard_id.clone().into()]);
                                }
                            }
                            map
                        });
                    region.cast_ns(ShardHomes { homes: shards_sub_map });
                });
        }
    }

    fn shutdown_shards(&mut self, context: &mut ActorContext, shutting_down_region: ActorRef, shards: HashSet<ImShardId>) -> anyhow::Result<()> {
        if shards.is_empty().not() {
            let shards_str = shards.iter().join(", ");
            info!("{}: Starting shutting down shards [{}] due to region shutting down or explicit stopping of shards.", self.type_name, shards_str);
            for shard in shards {
                self.start_shard_rebalance_if_needed(
                    context,
                    shard,
                    shutting_down_region.clone(),
                    self.settings.handoff_timeout,
                    false,
                )?;
            }
        }
        Ok(())
    }

    fn start_shard_rebalance_if_needed(
        &mut self,
        context: &mut ActorContext,
        shard: ImShardId,
        from: ActorRef,
        handoff_timeout: Duration,
        is_rebalance: bool,
    ) -> anyhow::Result<()> {
        if let Entry::Vacant(v) = self.rebalance_in_progress.entry(shard.clone()) {
            v.insert(HashSet::new());
            let regions = self.state.regions.keys().map(|region| region.clone()).collect::<HashSet<_>>();
            let region_proxies = self.state.region_proxies.clone();
            let regions = regions
                .union(&self.state.region_proxies)
                .into_iter()
                .map(|region| region.clone())
                .collect::<HashSet<_>>();
            let worker = context.spawn_anonymous(
                Self::rebalance_worker_props(
                    self.type_name.clone(),
                    shard,
                    from,
                    handoff_timeout,
                    regions,
                    is_rebalance,
                ),
            )?;
            self.rebalance_workers.insert(worker);
        }
        Ok(())
    }

    fn rebalance_worker_props(
        type_name: String,
        shard: ImShardId,
        shard_region_from: ActorRef,
        handoff_timeout: Duration,
        regions: HashSet<ActorRef>,
        is_rebalance: bool,
    ) -> Props {
        Props::new_with_ctx(move |context| {
            RebalanceWorker::new(
                context,
                type_name,
                shard,
                shard_region_from,
                handoff_timeout,
                regions,
                is_rebalance,
            )
        })
    }
}