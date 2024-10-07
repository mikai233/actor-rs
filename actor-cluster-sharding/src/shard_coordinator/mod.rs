use std::collections::BTreeSet;
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::sync::Arc;
use std::time::Duration;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use async_trait::async_trait;
use imstr::ImString;
use itertools::Itertools;
use tracing::{debug, error, info};

use actor_cluster::cluster::Cluster;
use actor_cluster::etcd_actor::keep_alive::KeepAlive;
use actor_cluster::etcd_client::PutOptions;
use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor::props::Props;
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt, PROVIDER};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::{decode_bytes, encode_bytes};

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_coordinator::coordinator_state::CoordinatorState;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::rebalance_tick::RebalanceTick;
use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_coordinator::rebalance_worker::shard_region_terminated::ShardRegionTerminated;
use crate::shard_coordinator::resend_shard_host::ResendShardHost;
use crate::shard_coordinator::state::{BinState, State};
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_coordinator::update_failed::UpdateFailed;
use crate::shard_region::{ImShardId, ShardId};
use crate::shard_region::host_shard::HostShard;
use crate::shard_region::shard_home::ShardHome;
use crate::shard_region::shard_homes::ShardHomes;

pub(crate) mod get_shard_home;
pub(crate) mod terminate_coordinator;
pub(crate) mod register;
pub(crate) mod register_proxy;
pub(crate) mod shard_started;
pub(crate) mod rebalance_worker;
mod rebalance_done;
mod state;
pub(crate) mod graceful_shutdown_req;
mod rebalance_tick;
mod rebalance_result;
mod stop_shard_timeout;
mod stop_shards;
mod shard_region_terminated;
mod state_update;
mod coordinator_state;
mod shard_region_proxy_terminated;
mod resend_shard_host;
mod allocate_shard_result;
pub(crate) mod region_stopped;
mod update_failed;

const SHARD_COORDINATOR_RETRY_DELAY: Duration = Duration::from_secs(3);
const SHARD_COORDINATOR_LEASE_TTL: i64 = 30;
const SHARD_COORDINATOR_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct ShardCoordinator {
    type_name: ImString,
    settings: Arc<ClusterShardingSettings>,
    allocation_strategy: Box<dyn ShardAllocationStrategy>,
    ignore_ref: ActorRef,
    cluster: Cluster,
    timers: Timers,
    min_members: usize,
    all_regions_registered: bool,
    state: State,
    state_lease_id: i64,
    coordinator_state: CoordinatorState,
    preparing_for_shutdown: bool,
    rebalance_in_progress: HashMap<ImShardId, HashSet<ActorRef>>,
    rebalance_workers: HashSet<ActorRef>,
    un_acked_host_shards: HashMap<ImShardId, ScheduleKey>,
    graceful_shutdown_in_progress: HashSet<ActorRef>,
    waiting_for_local_region_to_terminate: bool,
    alive_regions: HashSet<ActorRef>,
    region_termination_in_progress: HashSet<ActorRef>,
    waiting_for_shards_to_stop: HashMap<ImShardId, HashSet<(ActorRef, uuid::Uuid)>>,
    rebalance_tick_key: Option<ScheduleKey>,
    get_shard_home_requests: BTreeSet<(ActorRef, GetShardHome)>,
}

impl ShardCoordinator {
    pub(crate) fn new(
        context: &mut ActorContext1,
        type_name: ImString,
        settings: Arc<ClusterShardingSettings>,
        allocation_strategy: Box<dyn ShardAllocationStrategy>,
    ) -> anyhow::Result<Self> {
        let cluster = Cluster::get(context.system()).clone();
        let timers = Timers::new(context)?;
        let coordinator = Self {
            type_name,
            settings,
            allocation_strategy,
            ignore_ref: context.system().provider().ignore_ref().clone(),
            cluster,
            timers,
            min_members: 1,
            all_regions_registered: false,
            state: Default::default(),
            state_lease_id: 0,
            coordinator_state: Default::default(),
            preparing_for_shutdown: false,
            rebalance_in_progress: Default::default(),
            rebalance_workers: Default::default(),
            un_acked_host_shards: Default::default(),
            graceful_shutdown_in_progress: Default::default(),
            waiting_for_local_region_to_terminate: false,
            alive_regions: Default::default(),
            region_termination_in_progress: Default::default(),
            waiting_for_shards_to_stop: Default::default(),
            rebalance_tick_key: None,
            get_shard_home_requests: Default::default(),
        };
        Ok(coordinator)
    }
}

#[async_trait]
impl Actor for ShardCoordinator {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        self.timers.start_timer_with_fixed_delay(
            None,
            self.settings.rebalance_interval,
            RebalanceTick,
            context.myself().clone(),
        );
        let mut client = self.cluster.etcd_client();
        let lease_id = loop {
            match client.lease_grant(SHARD_COORDINATOR_LEASE_TTL, None).await {
                Ok(resp) => {
                    break resp.id();
                }
                Err(error) => {
                    error!("lease error {:?}", error);
                    tokio::time::sleep(SHARD_COORDINATOR_RETRY_DELAY).await;
                }
            }
        };
        self.state_lease_id = lease_id;
        //TODO adapter
        self.cluster.etcd_actor().cast_ns(KeepAlive {
            id: lease_id,
            applicant: context.myself().clone(),
            interval: SHARD_COORDINATOR_KEEPALIVE_INTERVAL,
        });
        let kvs = loop {
            match client.get(self.persistent_key(), None).await {
                Ok(mut resp) => {
                    break resp.take_kvs();
                }
                Err(error) => {
                    error!("ShardCoordinator get state from etcd error {:?}", error);
                    tokio::time::sleep(SHARD_COORDINATOR_RETRY_DELAY).await;
                }
            }
        };
        if let Some(kv) = kvs.first() {
            PROVIDER.sync_scope(self.cluster.system()?.provider_full(), || {
                match decode_bytes::<BinState>(kv.value()) {
                    Ok(state) => {
                        self.state = state.into();
                    }
                    Err(error) => {
                        error!("ShardCoordinator decode state error {:?}", error);
                    }
                }
            })
        }
        self.update(context, None).await;
        //TODO 从etcd获取持久化状态
        self.coordinator_state = CoordinatorState::Active;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        debug!("{}: ShardCoordinator {} stopped", self.type_name, context.myself());
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

impl ShardCoordinator {
    fn clear_rebalance_in_progress(&mut self, context: &mut ActorContext1, shard: ShardId) {
        if let Some(pending_get_shard_home) = self.rebalance_in_progress.remove(shard.as_str()) {
            let msg = GetShardHome { shard };
            let myself = context.myself();
            for get_shard_home_sender in pending_get_shard_home {
                myself.cast(msg.clone(), Some(get_shard_home_sender));
            }
        }
    }

    fn is_member(&self, context: &mut ActorContext1, region: &ActorRef) -> bool {
        let region_address = region.path().address();
        region_address == context.myself().path().address() || self.cluster.state().is_member_up(region_address)
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

    fn shutdown_shards(
        &mut self,
        context: &mut ActorContext1,
        shutting_down_region: ActorRef,
        shards: HashSet<ImShardId>,
    ) -> anyhow::Result<()> {
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

    fn continue_rebalance(&mut self, context: &mut ActorContext1, shards: HashSet<ImShardId>) -> anyhow::Result<()> {
        if shards.is_empty().not() || self.rebalance_in_progress.is_empty().not() {
            let shards_str = shards.iter().join(", ");
            let rebalance_str = self.rebalance_in_progress.keys().join(", ");
            info!(
                "{}: Starting rebalance for shards [{}]. Current shards rebalancing: [{}]",
                self.type_name,
                shards_str,
                rebalance_str,
            );
        }
        for shard in shards {
            if !self.rebalance_in_progress.contains_key(&shard) {
                match self.state.shards.get(&shard) {
                    None => {
                        debug!("{}: Rebalance of non-existing shard [{}] is ignored", self.type_name, shard);
                    }
                    Some(rebalance_from_region) => {
                        debug!("{}: Rebalance shard [{}] from [{}]", self.type_name, shard, rebalance_from_region);
                        self.start_shard_rebalance_if_needed(
                            context,
                            shard,
                            rebalance_from_region.clone(),
                            self.settings.handoff_timeout,
                            true,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn start_shard_rebalance_if_needed(
        &mut self,
        context: &mut ActorContext1,
        shard: ImShardId,
        from: ActorRef,
        handoff_timeout: Duration,
        is_rebalance: bool,
    ) -> anyhow::Result<()> {
        if let Entry::Vacant(v) = self.rebalance_in_progress.entry(shard.clone()) {
            v.insert(HashSet::new());
            let regions = self.state.regions.keys().map(|region| region.clone()).collect::<HashSet<_>>();
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
        type_name: ImString,
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

    fn terminate(&mut self, context: &mut ActorContext1) {
        //TODO
        if self.region_termination_in_progress.is_empty() {
            debug!("{}: Received termination message.", self.type_name);
        } else {
            debug!(
                "{}: Received termination message. Rebalance in progress of [{}] shards.",
                self.type_name,
                self.rebalance_in_progress.len(),
            );
        }
        context.stop(context.myself());
    }

    async fn region_terminated(&mut self, context: &mut ActorContext1, region: ActorRef) {
        for worker in &self.rebalance_workers {
            worker.cast_ns(ShardRegionTerminated { region: region.clone() });
        }
        if let Some(shards) = self.state.regions.get(&region) {
            let gracefully = if self.graceful_shutdown_in_progress.contains(&region) {
                " (gracefully)"
            } else {
                ""
            };
            debug!("{}: ShardRegion terminated{}: [{}]", self.type_name, gracefully, region);
            self.region_termination_in_progress.insert(region.clone());
            for shard in shards {
                context.myself().cast(
                    GetShardHome { shard: shard.clone().into() },
                    Some(self.ignore_ref.clone()),
                );
            }
            self.update_state(context, ShardState::ShardRegionTerminated { region: region.clone() }).await;
            self.graceful_shutdown_in_progress.remove(&region);
            self.region_termination_in_progress.remove(&region);
            self.alive_regions.remove(&region);
            //TODO
        }
    }

    async fn region_proxy_terminated(&mut self, context: &mut ActorContext1, proxy: ActorRef) {
        for worker in &self.rebalance_workers {
            worker.cast_ns(ShardRegionTerminated { region: proxy.clone() });
        }
        if self.state.region_proxies.contains(&proxy) {
            debug!("{}: ShardRegion proxy terminated: [{}]", self.type_name, proxy);
            self.update_state(context, ShardState::ShardRegionProxyTerminated { region_proxy: proxy }).await;
        }
    }

    async fn update_state(&mut self, context: &mut ActorContext1, state: ShardState) {
        self.update(context, Some(state)).await;
    }

    async fn update(&mut self, context: &mut ActorContext1, state: Option<ShardState>) {
        if let Some(state) = state {
            self.state.updated(state);
        }
        let bin_state = PROVIDER.sync_scope(self.cluster.system().unwrap().provider_full(), || {
            encode_bytes(&self.state.bin_state())
        });
        match bin_state {
            Ok(bytes) => {
                let mut client = self.cluster.etcd_client();
                let put_opts = PutOptions::new().with_lease(self.state_lease_id);
                if let Some(error) = client.put(self.persistent_key(), bytes, Some(put_opts)).await.err() {
                    error!("put ShardCoordinator state to etcd error {:?}", error);
                    self.timers.start_single_timer(SHARD_COORDINATOR_RETRY_DELAY, UpdateFailed, context.myself().clone());
                }
            }
            Err(error) => {
                error!("ShardCoordinator state {} serialize error {:?}", self.state, error);
                self.timers.start_single_timer(SHARD_COORDINATOR_RETRY_DELAY, UpdateFailed, context.myself().clone());
            }
        }
    }

    fn send_host_shard_msg(&mut self, context: &mut ActorContext1, shard: ImShardId, region: ActorRef) {
        region.cast(HostShard { shard: shard.clone().into() }, Some(context.myself().clone()));
        let resend_shard_host = ResendShardHost {
            shard: shard.clone(),
            region,
        };
        let key = self.timers.start_single_timer(
            self.settings.shard_start_timeout,
            resend_shard_host,
            context.myself().clone(),
        );
        self.un_acked_host_shards.insert(shard, key);
    }

    fn handle_get_shard_home(&mut self, context: &mut ActorContext1, sender: ActorRef, shard: ImShardId) -> bool {
        if self.rebalance_in_progress.contains_key(&shard) {
            self.defer_get_shard_home_request(shard, sender);
            self.unstash_one_get_shard_home_request(context);
            true
        } else if !self.has_all_regions_registered() {
            debug!(
                "{}: GetShardHome [{}] request from [{}] ignored, because not all regions have registered yet.",
                self.type_name,
                shard,
                sender,
            );
            true
        } else {
            match self.state.shards.get(&shard) {
                None => {
                    false
                }
                Some(shard_region_ref) => {
                    if self.region_termination_in_progress.contains(&shard_region_ref) {
                        debug!(
                            "{}: GetShardHome [{}] request ignored, due to region [{}] termination in progress",
                            self.type_name,
                            shard, shard_region_ref,
                        );
                    } else {
                        sender.cast_ns(ShardHome { shard: shard.into(), shard_region: shard_region_ref.clone() });
                    }
                    self.unstash_one_get_shard_home_request(context);
                    true
                }
            }
        }
    }

    fn has_all_regions_registered(&mut self) -> bool {
        if self.all_regions_registered {
            true
        } else {
            self.all_regions_registered = self.alive_regions.len() >= self.min_members;
            self.all_regions_registered
        }
    }

    fn defer_get_shard_home_request(&mut self, shard: ImShardId, from: ActorRef) {
        debug!(
            "{}: GetShardHome [{}] request from [{}] deferred, because rebalance is in progress for this shard. \
            It will be handled when reblance is done.",
            self.type_name,
            shard,
            from,
        );
        match self.rebalance_in_progress.entry(shard) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(from);
            }
            Entry::Vacant(v) => {
                let mut refs = HashSet::new();
                refs.insert(from);
                v.insert(refs);
            }
        }
    }

    fn stash_get_shard_home_request(&mut self, sender: ActorRef, request: GetShardHome) {
        debug!(
            "{}: GetShardHome [{}] request from [{}] stashed, because waiting for initial state or update of state. \
            It will be handled afterwards.",
            self.type_name,
            request.shard,
            sender,
        );
        self.get_shard_home_requests.insert((sender, request));
    }

    fn unstash_one_get_shard_home_request(&mut self, context: &mut ActorContext1) {
        if let Some((sender, request)) = self.get_shard_home_requests.pop_first() {
            context.myself().tell(request.into_dyn(), Some(sender));
        }
    }

    async fn continue_get_shard_home(&mut self, context: &mut ActorContext1, shard: ImShardId, region: ActorRef, get_shard_home_sender: ActorRef) {
        if self.rebalance_in_progress.contains_key(&shard) {
            self.defer_get_shard_home_request(shard, get_shard_home_sender);
        } else {
            match self.state.shards.get(&shard) {
                None => {
                    if self.state.regions.contains_key(&region) &&
                        !self.graceful_shutdown_in_progress.contains(&region) &&
                        !self.region_termination_in_progress.contains(&region) {
                        self.update_state(context, ShardState::ShardHomeAllocated { shard: shard.clone(), region: region.clone() }).await;
                        debug!("{}: Shard [{}] allocated at [{}]", self.type_name, shard, region);
                        self.send_host_shard_msg(context, shard.clone(), region.clone());
                        get_shard_home_sender.cast_ns(ShardHome { shard: shard.into(), shard_region: region });
                    } else {
                        debug!(
                            "{}: Allocated region [{}] for shard [{}] is not (any longer) one of the registered regions: {:?}",
                            self.type_name,
                            region,
                            shard,
                            self.state,
                        );
                    }
                }
                Some(region) => {
                    get_shard_home_sender.cast_ns(ShardHome { shard: shard.into(), shard_region: region.clone() });
                }
            }
        }
    }

    fn persistent_key(&self) -> String {
        let name = &self.cluster.system().unwrap().name;
        format!("actor/{}/cluster/shard_coordinator/{}/state", name, self.type_name)
    }
}