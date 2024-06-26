use std::collections::hash_map::Entry;
use std::ops::{Deref, Mul, Not};
use std::sync::Arc;
use std::time::Duration;

use ahash::{HashMap, HashSet, HashSetExt};
use anyhow::anyhow;
use async_trait::async_trait;
use imstr::ImString;
use itertools::Itertools;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, info, warn};

use actor_cluster::cluster::Cluster;
use actor_cluster::member::{Member, MemberStatus};
use actor_cluster::unique_address::UniqueAddress;
use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::context::{ActorContext, Context, ContextExt};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION};
use actor_core::actor::dead_letter_listener::DeadMessage;
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::message_buffer::{BufferEnvelope, MessageBufferMap};
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardEnvelope};
use crate::shard::Shard;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::graceful_shutdown_req::GracefulShutdownReq;
use crate::shard_coordinator::region_stopped::RegionStopped;
use crate::shard_coordinator::register::Register;
use crate::shard_coordinator::register_proxy::RegisterProxy;
use crate::shard_region::cluster_event::ClusterEventWrap;
use crate::shard_region::deliver_target::DeliverTarget;
use crate::shard_region::graceful_shutdown::GracefulShutdown;
use crate::shard_region::register_retry::RegisterRetry;
use crate::shard_region::retry::Retry;
use crate::shard_region::shard_region_buffer_envelope::ShardRegionBufferEnvelope;
use crate::shard_region::shard_region_terminated::ShardRegionTerminated;
use crate::shard_region::shard_terminated::ShardTerminated;

mod shard_region_buffer_envelope;
mod shard_terminated;
pub(crate) mod handoff;
mod register_retry;
mod retry;
mod cluster_event;
mod shard_envelope;
pub(crate) mod host_shard;
pub(crate) mod shard_home;
mod shard_region_terminated;
pub(crate) mod shard_homes;
pub(crate) mod register_ack;
mod coordinator_terminated;
pub(crate) mod begin_handoff;
pub(crate) mod shard_initialized;
mod deliver_target;
mod graceful_shutdown;
mod graceful_shutdown_timeout;

pub type ShardId = String;

pub type ImShardId = ImString;

pub type EntityId = String;

pub type ImEntityId = ImString;

#[derive(Debug)]
pub struct ShardRegion {
    type_name: ImString,
    entity_props: Option<PropsBuilder<ImEntityId>>,
    settings: Arc<ClusterShardingSettings>,
    coordinator_path: String,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    timers: Timers,
    regions: HashMap<ActorRef, HashSet<ImShardId>>,
    region_by_shard: HashMap<ImShardId, ActorRef>,
    shard_buffers: MessageBufferMap<ImShardId, ShardRegionBufferEnvelope>,
    shards: HashMap<ImShardId, ActorRef>,
    shards_by_ref: HashMap<ActorRef, ImShardId>,
    starting_shards: HashSet<ImShardId>,
    handing_off: HashSet<ActorRef>,
    graceful_shutdown_in_progress: bool,
    preparing_for_shutdown: bool,
    retry_count: usize,
    init_registration_delay: Duration,
    next_registration_delay: Duration,
    graceful_shutdown_progress: Sender<()>,
    cluster: Cluster,
    register_retry_key: Option<ScheduleKey>,
    members: HashMap<UniqueAddress, Member>,
    coordinator: Option<ActorRef>,
}

impl ShardRegion {
    fn new(
        context: &mut ActorContext,
        type_name: ImString,
        entity_props: Option<PropsBuilder<ImEntityId>>,
        settings: Arc<ClusterShardingSettings>,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> anyhow::Result<Self> {
        let timers = Timers::new(context)?;
        let cluster = Cluster::get(context.system()).clone();
        let (graceful_shutdown_progress_tx, mut graceful_shutdown_progress_rx) = channel(1);
        let graceful_shutdown_progress_tx_clone = graceful_shutdown_progress_tx.clone();
        let myself = context.myself().clone();
        CoordinatedShutdown::get(context.system())
            .add_task(context.system(), PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION, "region_shutdown", async move {
                if cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed {
                    let _ = graceful_shutdown_progress_tx_clone.send(()).await;
                } else {
                    myself.cast_ns(GracefulShutdown);
                    graceful_shutdown_progress_rx.recv().await;
                }
            })?;
        let cluster = Cluster::get(context.system()).clone();
        let myself = Self {
            type_name,
            entity_props,
            settings,
            coordinator_path,
            extractor,
            handoff_stop_message,
            timers,
            regions: Default::default(),
            region_by_shard: Default::default(),
            shard_buffers: Default::default(),
            shards: Default::default(),
            shards_by_ref: Default::default(),
            starting_shards: Default::default(),
            handing_off: Default::default(),
            graceful_shutdown_in_progress: false,
            preparing_for_shutdown: false,
            retry_count: 0,
            init_registration_delay: Duration::from_secs(1),
            next_registration_delay: Duration::from_secs(1),
            graceful_shutdown_progress: graceful_shutdown_progress_tx,
            cluster,
            register_retry_key: None,
            members: Default::default(),
            coordinator: None,
        };
        Ok(myself)
    }

    pub(crate) fn props(
        type_name: ImString,
        entity_props: PropsBuilder<ImEntityId>,
        settings: Arc<ClusterShardingSettings>,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> Props {
        debug_assert!(handoff_stop_message.cloneable(), "message {} is not cloneable", handoff_stop_message.name());
        Props::new_with_ctx(move |context| {
            Self::new(
                context,
                type_name,
                Some(entity_props),
                settings,
                coordinator_path,
                extractor,
                handoff_stop_message,
            )
        })
    }

    pub(crate) fn proxy_props(
        type_name: ImString,
        settings: Arc<ClusterShardingSettings>,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
    ) -> Props {
        Props::new_with_ctx(move |context| {
            Self::new(
                context,
                type_name,
                None,
                settings,
                coordinator_path,
                extractor,
                DynMessage::system(PoisonPill),
            )
        })
    }

    fn start_registration(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.next_registration_delay = self.init_registration_delay;
        self.register(context)?;
        self.scheduler_next_registration(context);
        Ok(())
    }

    fn register(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let actor_selections = self.coordinator_selection(context)?;
        for selection in &actor_selections {
            selection.tell(self.registration_message(context), ActorRef::no_sender());
        }
        if self.shard_buffers.is_empty().not() && self.retry_count >= 5 {
            if actor_selections.is_empty().not() {
                let all_up_members = self.members.values().
                    filter(|m| matches!(m.status, MemberStatus::Up))
                    .join(", ");
                let buffer_size = self.shard_buffers.total_size();
                let type_name = &self.type_name;
                let selections_str = actor_selections
                    .iter()
                    .map(|s| s.to_string())
                    .join(", ");
                if buffer_size > 0 {
                    warn!(
                        "{}: Trying to register to coordinator at [{}], but no acknowledgement. Total [{}] buffered messages. All up members {}",
                        type_name,
                        selections_str,
                        buffer_size,
                        all_up_members,
                    )
                } else {
                    debug!(
                        "{}: Trying to register to coordinator at [{}], but no acknowledgement. No buffered messages yet. All up members {}",
                        type_name,
                        selections_str,
                        all_up_members,
                    )
                }
            } else {
                let part_of_cluster = self.cluster.self_member().status != MemberStatus::Removed;
                let possible_reason = if part_of_cluster {
                    "Has Cluster Sharding been started on every node and nodes been configured with the correct role(s)?"
                } else {
                    "Probably, node not join to cluster"
                };
                let buffer_size = self.shard_buffers.total_size();
                if buffer_size > 0 {
                    warn!("{}: No coordinator found to register. {} Total [{}] buffered messages.", self.type_name, possible_reason, buffer_size);
                } else {
                    debug!("{}: No coordinator found to register. {} No buffered messages yet.", self.type_name, possible_reason);
                }
            }
        }
        Ok(())
    }

    fn scheduler_next_registration(&mut self, context: &mut ActorContext) {
        if self.next_registration_delay < self.settings.retry_interval {
            let key = self.timers.start_single_timer(
                self.next_registration_delay,
                RegisterRetry,
                context.myself().clone(),
            );
            self.register_retry_key = Some(key);
            self.next_registration_delay = self.next_registration_delay.mul(2);
        }
    }

    fn finish_registration(&mut self) {
        if let Some(key) = self.register_retry_key.take() {
            key.cancel();
        }
    }

    fn coordinator_selection(&self, context: &mut ActorContext) -> anyhow::Result<Vec<ActorSelection>> {
        let mut selections = vec![];
        for member in self.members.values() {
            if matches!(member.status, MemberStatus::Up) {
                let path = RootActorPath::new(member.address().clone(), "/")
                    .descendant(self.coordinator_path.split("/"));
                let selection = context.actor_selection(ActorSelectionPath::FullPath(path))?;
                selections.push(selection);
            }
        }
        Ok(selections)
    }

    fn registration_message(&self, context: &mut ActorContext) -> DynMessage {
        let myself = context.myself().clone();
        if self.entity_props.is_some() {
            DynMessage::user(Register { shard_region: myself })
        } else {
            DynMessage::user(RegisterProxy { shard_region_proxy: myself })
        }
    }

    fn deliver_message(&mut self, context: &mut ActorContext, envelope: ShardEnvelope<ShardRegion>) -> anyhow::Result<()> {
        let shard_id = self.extractor.shard_id(&envelope);
        let type_name = &self.type_name;
        match self.region_by_shard.get_key_value(shard_id.as_str()) {
            None => {
                if !self.shard_buffers.contains_key(shard_id.as_str()) {
                    match &self.coordinator {
                        None => {
                            debug!("{type_name}: Request shard [{shard_id}] home, Coordinator [None]");
                        }
                        Some(coordinator) => {
                            debug!("{type_name}: Request shard [{shard_id}] home, Coordinator [{coordinator}]");
                            coordinator.cast(GetShardHome { shard: shard_id.clone() }, Some(context.myself().clone()));
                        }
                    }
                }
                self.buffer_message(context, shard_id.into(), envelope, context.sender().cloned());
            }
            Some((shard_id, shard_region_ref)) if shard_region_ref == context.myself() => {
                let shard_id = shard_id.clone();
                match self.get_shard(context, shard_id.clone())? {
                    None => {
                        self.buffer_message(context, shard_id, envelope, context.sender().cloned());
                    }
                    Some(shard) => {
                        if self.shard_buffers.contains_key(shard_id.as_str()) {
                            self.buffer_message(context, shard_id.clone(), envelope, context.sender().cloned());
                            self.deliver_buffered_messages(&shard_id, DeliverTarget::Shard(&shard));
                        } else {
                            context.forward(&shard, envelope.into_shard_envelope().into_dyn());
                        }
                    }
                }
            }
            Some((shard_id, shard_region_ref)) => {
                debug!("{type_name}: Forwarding message for shard [{shard_id}] to [{shard_region_ref}]");
                context.forward(shard_region_ref, envelope.into_dyn());
            }
        }
        Ok(())
    }

    fn buffer_message(
        &mut self,
        context: &mut ActorContext,
        shard_id: ImShardId,
        msg: ShardEnvelope<ShardRegion>,
        sender: Option<ActorRef>,
    ) {
        let total_buf_size = self.shard_buffers.total_size();
        let buffer_size = self.settings.buffer_size;
        let type_name = &self.type_name;
        if total_buf_size >= buffer_size {
            warn!("{type_name}: Buffer is full, dropping message for shard [{shard_id}]");
            context.system().dead_letters().cast_ns(DeadMessage(msg.into_dyn()));
        } else {
            let envelop = ShardRegionBufferEnvelope {
                message: msg,
                sender,
            };
            self.shard_buffers.push(shard_id, envelop);
            let total = total_buf_size + 1;
            if total % (buffer_size / 10) == 0 {
                let cap = 100.0 * total as f64 / buffer_size as f64;
                let log_msg = format!("{type_name}: ShardRegion is using [{cap} %] of its buffer capacity.");
                if total <= buffer_size / 2 {
                    info!(log_msg);
                } else {
                    warn!("{} The coordinator might not be available. You might want to check cluster membership status.", log_msg);
                }
            }
        }
    }

    fn deliver_buffered_messages(&mut self, shard_id: &ImShardId, target: DeliverTarget) {
        if self.shard_buffers.contains_key(shard_id) {
            if let Some(buffers) = self.shard_buffers.remove(shard_id) {
                let type_name = &self.type_name;
                let buf_size = buffers.len();
                debug!("{type_name}: Deliver [{buf_size}] buffered messages for shard [{shard_id}]");
                for envelope in buffers {
                    let (msg, sender) = envelope.into_inner();
                    match target {
                        DeliverTarget::Shard(receiver) => {
                            receiver.cast(msg.into_shard_envelope(), sender);
                        }
                        DeliverTarget::ShardRegion(receiver) => {
                            receiver.cast(msg, sender);
                        }
                    }
                }
            }
        }
        self.retry_count = 0;
    }

    fn get_shard(&mut self, context: &mut ActorContext, id: ImShardId) -> anyhow::Result<Option<ActorRef>> {
        if self.starting_shards.contains(&id) {
            Ok(None)
        } else {
            if let Some(shard) = self.shards.get(&id) {
                Ok(Some(shard.clone()))
            } else {
                match &self.entity_props {
                    None => {
                        panic!("Shard must not be allocated to a proxy only ShardRegion");
                    }
                    Some(props_builder) if self.shards_by_ref.values().find(|r| **r == id).is_none() => {
                        debug!("{}: Starting shard [{}] in region", self.type_name, id);
                        let shard_props = Shard::props(
                            self.type_name.clone(),
                            id.clone(),
                            props_builder.clone(),
                            self.settings.clone(),
                            self.extractor.clone(),
                            self.handoff_stop_message.dyn_clone()?,
                        );
                        let shard = context.spawn(shard_props, id.deref())?;
                        context.watch(shard.clone(), ShardTerminated::new)?;
                        self.shards_by_ref.insert(shard.clone(), id.clone());
                        self.shards.insert(id.clone(), shard.clone());
                        self.starting_shards.insert(id);
                        //TODO passivation strategy
                        Ok(None)
                    }
                    Some(_) => Ok(None)
                }
            }
        }
    }

    fn try_request_shard_buffer_homes(&self, context: &mut ActorContext) {
        self.coordinator.foreach(|coord| {
            let mut total_buffered = 0;
            let mut shards = vec![];
            self.shard_buffers.iter().for_each(|(shard, buf)| {
                total_buffered += buf.len();
                shards.push(shard);
                debug!(
                    "{}: Requesting shard home for [{}] from coordinator at [{}]. [{}] buffered messages.",
                    self.type_name,
                    shard,
                    coord,
                    buf.len(),
                );
                coord.cast(GetShardHome { shard: shard.clone().into() }, Some(context.myself().clone()));
            });
            if self.retry_count >= 5 && self.retry_count % 5 == 0 {
                let shards_str = shards.iter().map(|shard| shard.as_str()).join(", ");
                warn!(
                    "{}: Requested shard homes [{}] from coordinator at [{}]. [{}] total buffered messages.",
                    self.type_name,
                    shards_str,
                    coord,
                    total_buffered,
                );
            }
        });
    }

    fn send_graceful_shutdown_to_coordinator_if_in_progress(&self, context: &mut ActorContext) -> anyhow::Result<()> {
        if self.graceful_shutdown_in_progress {
            let actor_selections = self.coordinator_selection(context)?;
            let selection_str = actor_selections.iter().map(|selection| selection.to_string()).join(", ");
            debug!("{}: Sending graceful shutdown to {}", self.type_name, selection_str);
            for selection in actor_selections {
                selection.tell(GracefulShutdownReq { shard_region: context.myself().clone() }.into_dyn(), ActorRef::no_sender());
            }
        }
        Ok(())
    }

    fn try_complete_graceful_shutdown_if_in_progress(&self, context: &mut ActorContext) {
        if self.graceful_shutdown_in_progress && self.shards.is_empty() && self.shard_buffers.is_empty() {
            debug!("{}: Completed graceful shutdown of region.", self.type_name);
            context.stop(context.myself());
        }
    }

    fn receive_shard_home(&mut self, context: &mut ActorContext, shard: ImShardId, shard_region_ref: ActorRef) -> anyhow::Result<()> {
        let type_name = &self.type_name;
        debug!("{type_name}: Shard [{shard}] located at [{shard_region_ref}]");
        if let Some(r) = self.region_by_shard.get(&shard) {
            if r == context.myself() && &shard_region_ref != context.myself() {
                return Err(anyhow!("{type_name}: Unexpected change of shard [{shard}] from self to [{shard_region_ref}]"));
            }
        }
        self.region_by_shard.insert(shard.clone(), shard_region_ref.clone());
        match self.regions.entry(shard_region_ref.clone()) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(shard.clone());
            }
            Entry::Vacant(v) => {
                let mut shards = HashSet::new();
                shards.insert(shard.clone());
                v.insert(shards);
            }
        }
        if &shard_region_ref != context.myself() {
            if context.is_watching(&shard_region_ref).not() {
                context.watch(shard_region_ref.clone(), ShardRegionTerminated::new)?;
            }
        }
        if &shard_region_ref == context.myself() {
            self.get_shard(context, shard.clone())?.foreach(|region| {
                self.deliver_buffered_messages(&shard, DeliverTarget::ShardRegion(region));
            });
        } else {
            self.deliver_buffered_messages(&shard, DeliverTarget::ShardRegion(&shard_region_ref));
        }
        Ok(())
    }
}

#[async_trait]
impl Actor for ShardRegion {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.subscribe_cluster_event(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        self.timers.start_timer_with_fixed_delay(
            None,
            self.settings.retry_interval,
            Retry,
            context.myself().clone(),
        );
        self.start_registration(context)?;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{}: Region {} stopped", self.type_name, context.myself());
        self.cluster.unsubscribe_cluster_event(context.myself())?;
        self.coordinator.foreach(|coordinator| {
            coordinator.cast_ns(RegionStopped { shard_region: context.myself().clone() });
        });
        let _ = self.graceful_shutdown_progress.send(()).await;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
