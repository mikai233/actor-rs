use std::collections::{HashMap, HashSet};
use std::ops::{Mul, Not};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use actor_cluster::cluster::Cluster;
use actor_cluster::member::{Member, MemberStatus};
use actor_cluster::unique_address::UniqueAddress;
use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::message::message_buffer::{BufferEnvelope, MessageBufferMap};
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardEntityEnvelope};
use crate::shard::Shard;
use crate::shard::shard_envelope::ShardEnvelope;
use crate::shard_coordinator::get_shard_home::GetShardHome;
use crate::shard_coordinator::register::Register;
use crate::shard_coordinator::register_proxy::RegisterProxy;
use crate::shard_region::cluster_event_wrap::ClusterEventWrap;
use crate::shard_region::register_retry::RegisterRetry;
use crate::shard_region::retry::Retry;
use crate::shard_region::shard_region_buffer_envelope::ShardRegionBufferEnvelope;
use crate::shard_region::shard_terminated::ShardTerminated;

mod shard_region_buffer_envelope;
mod shard_terminated;
mod handoff;
mod register_retry;
mod retry;
mod cluster_event_wrap;
mod shard_entity_envelope;

pub type ShardId = String;

pub type EntityId = String;

pub struct ShardRegion {
    type_name: String,
    entity_props: Option<Arc<PropsBuilderSync<EntityId>>>,
    settings: Arc<ClusterShardingSettings>,
    coordinator_path: String,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    timers: Timers,
    regions: HashMap<ActorRef, HashSet<ShardId>>,
    region_by_shard: HashMap<ShardId, ActorRef>,
    shard_buffers: MessageBufferMap<ShardId, ShardRegionBufferEnvelope>,
    shards: HashMap<ShardId, ActorRef>,
    shards_by_ref: HashMap<ActorRef, ShardId>,
    starting_shards: HashSet<ShardId>,
    handing_off: HashSet<ActorRef>,
    retry_count: usize,
    init_registration_delay: Duration,
    next_registration_delay: Duration,
    cluster: Cluster,
    cluster_event_adapter: ActorRef,
    register_retry_key: Option<ScheduleKey>,
    members: HashMap<UniqueAddress, Member>,
    coordinator: Option<ActorRef>,
}

impl ShardRegion {
    fn new(
        context: &mut ActorContext,
        type_name: String,
        entity_props: Option<Arc<PropsBuilderSync<EntityId>>>,
        settings: Arc<ClusterShardingSettings>,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> anyhow::Result<Self> {
        let timers = Timers::new(context)?;
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
            retry_count: 0,
            init_registration_delay: Duration::from_secs(1),
            next_registration_delay: Duration::from_secs(1),
            cluster,
            cluster_event_adapter: context.message_adapter(|m| { DynMessage::user(ClusterEventWrap(m)) }),
            register_retry_key: None,
            members: Default::default(),
            coordinator: None,
        };
        Ok(myself)
    }

    pub(crate) fn props(
        type_name: String,
        entity_props: Arc<PropsBuilderSync<EntityId>>,
        settings: Arc<ClusterShardingSettings>,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> Props {
        debug_assert!(handoff_stop_message.is_cloneable(), "message {} is not cloneable", handoff_stop_message.name());
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
        type_name: String,
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
                    .collect::<Vec<_>>();
                let buffer_size = self.shard_buffers.total_size();
                let type_name = &self.type_name;
                let selections_str = actor_selections
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>().join(", ");
                if buffer_size > 0 {
                    warn!(
                        "{}: Trying to register to coordinator at [{}], but no acknowledgement. Total [{}] buffered messages. All up members {:?}",
                        type_name,
                        selections_str,
                        buffer_size,
                        all_up_members,
                    )
                } else {
                    debug!(
                        "{}: Trying to register to coordinator at [{}], but no acknowledgement. No buffered messages yet. All up members {:?}",
                        type_name,
                        selections_str,
                        all_up_members,
                    )
                }
            } else {}
        }
        Ok(())
    }

    fn scheduler_next_registration(&mut self, context: &mut ActorContext) {
        if self.next_registration_delay < self.settings.retry_interval {
            let key = self.timers.start_single_timer(
                self.next_registration_delay,
                DynMessage::user(RegisterRetry),
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

    fn deliver_message(&mut self, context: &mut ActorContext, envelope: ShardEntityEnvelope) -> anyhow::Result<()> {
        let shard_id = self.extractor.shard_id(&envelope);
        let type_name = &self.type_name;
        match self.region_by_shard.get(&shard_id) {
            None => {
                if !self.shard_buffers.contains_key(&shard_id) {
                    match &self.coordinator {
                        None => {
                            debug!("{type_name}: Request shard [{shard_id}] home, Coordinator [None]");
                        }
                        Some(coordinator) => {
                            debug!("{type_name}: Request shard [{shard_id}] home, Coordinator [{coordinator}]");
                            coordinator.cast_ns(GetShardHome { shard: shard_id.clone() });
                        }
                    }
                }
                self.buffer_message(shard_id, envelope, context.sender().cloned());
            }
            Some(shard_region_ref) if shard_region_ref == context.myself() => {
                if let Some(shard) = self.get_shard(context, shard_id.clone())? {
                    if self.shard_buffers.contains_key(&shard_id) {
                        self.buffer_message(shard_id.clone(), envelope, context.sender().cloned());
                        self.deliver_buffered_messages(&shard_id, &shard);
                    } else {
                        shard.cast(ShardEnvelope(envelope), context.sender().cloned());
                    }
                }
            }
            Some(shard_region_ref) => {
                debug!("{type_name}: Forwarding message for shard [{shard_id}] to [{shard_region_ref}]");
                shard_region_ref.cast(envelope, context.sender().cloned());
            }
        }
        Ok(())
    }

    fn buffer_message(&mut self, shard_id: ShardId, msg: ShardEntityEnvelope, sender: Option<ActorRef>) {
        let total_buf_size = self.shard_buffers.total_size();
        //TODO buffer size
        let buffer_size = 5000;
        let type_name = &self.type_name;
        if total_buf_size >= buffer_size {
            warn!("{type_name}: Buffer is full, dropping message for shard [{shard_id}]");
            //TODO send to dead letter
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

    fn deliver_buffered_messages(&mut self, shard_id: &ShardId, receiver: &ActorRef) {
        if self.shard_buffers.contains_key(shard_id) {
            if let Some(buffers) = self.shard_buffers.remove(shard_id) {
                let type_name = &self.type_name;
                let buf_size = buffers.len();
                debug!("{type_name}: Deliver [{buf_size}] buffered messages for shard [{shard_id}]");
                for envelope in buffers {
                    let (msg, sender) = envelope.into_inner();
                    receiver.cast(ShardEnvelope(msg), sender);
                }
            }
        }
        self.retry_count = 0;
    }

    fn get_shard(&mut self, context: &mut ActorContext, id: ShardId) -> anyhow::Result<Option<ActorRef>> {
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
                        let shard = context.spawn(shard_props, id.clone())?;
                        context.watch(ShardTerminated(shard.clone()));
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
}

#[async_trait]
impl Actor for ShardRegion {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.subscribe_cluster_event(self.cluster_event_adapter.clone());
        self.timers.start_timer_with_fixed_delay(
            None,
            self.settings.retry_interval,
            DynMessage::user(Retry),
            context.myself().clone(),
        );

        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.unsubscribe_cluster_event(&self.cluster_event_adapter);
        Ok(())
    }
}
