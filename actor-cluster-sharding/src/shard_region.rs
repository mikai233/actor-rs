use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::ops::{Mul, Not};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{debug, info, warn};

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_event::ClusterEvent;
use actor_cluster::member::{Member, MemberStatus};
use actor_cluster::unique_address::UniqueAddress;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::message_buffer::{BufferEnvelope as TBufferEnvelope, MessageBufferMap};
use actor_core::message::poison_pill::PoisonPill;
use actor_core::message::terminated::Terminated;
use actor_derive::{EmptyCodec, MessageCodec};

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardEntityEnvelope};
use crate::shard::{Shard, ShardEnvelope};
use crate::shard_coordinator::{GetShardHome, Register, RegisterProxy, ShardStopped};

pub type ShardId = String;

pub type EntityId = String;

pub struct ShardRegion {
    type_name: String,
    entity_props: Option<Props>,
    settings: ClusterShardingSettings,
    coordinator_path: String,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    timers: Timers,
    regions: HashMap<ActorRef, HashSet<ShardId>>,
    region_by_shard: HashMap<ShardId, ActorRef>,
    shard_buffers: MessageBufferMap<ShardId, BufferEnvelope>,
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
        entity_props: Option<Props>,
        settings: ClusterShardingSettings,
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
        entity_props: Props,
        settings: ClusterShardingSettings,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> Props {
        debug_assert!(handoff_stop_message.is_cloneable(), "message {} is not cloneable", handoff_stop_message.name);
        let handoff_stop_message = Mutex::new(handoff_stop_message);
        Props::create(move |context| {
            let handoff_stop_message = handoff_stop_message.lock().unwrap().dyn_clone().unwrap();
            Self::new(
                context,
                type_name.clone(),
                Some(entity_props.clone()),
                settings.clone(),
                coordinator_path.clone(),
                extractor.clone(),
                handoff_stop_message,
            )
        })
    }

    pub(crate) fn proxy_props(
        type_name: String,
        settings: ClusterShardingSettings,
        coordinator_path: String,
        extractor: Box<dyn MessageExtractor>,
    ) -> Props {
        Props::create(move |context| {
            Self::new(
                context,
                type_name.clone(),
                None,
                settings.clone(),
                coordinator_path.clone(),
                extractor.clone(),
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
            let envelop = BufferEnvelope {
                envelope: msg,
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
            if let Some(buffers) = self.shard_buffers.remove_buffer(shard_id) {
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
                    Some(props) if self.shards_by_ref.values().find(|r| **r == id).is_none() => {
                        debug!("{}: Starting shard [{}] in region", self.type_name, id);
                        let shard_props = Shard::props(
                            self.type_name.clone(),
                            id.clone(),
                            props.clone(),
                            self.settings.clone(),
                            self.extractor.clone(),
                            self.handoff_stop_message.dyn_clone().into_result()?,
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

#[derive(Debug)]
struct BufferEnvelope {
    envelope: ShardEntityEnvelope,
    sender: Option<ActorRef>,
}

impl TBufferEnvelope for BufferEnvelope {
    type M = ShardEntityEnvelope;

    fn message(&self) -> &Self::M {
        &self.envelope
    }

    fn sender(&self) -> &Option<ActorRef> {
        &self.sender
    }

    fn into_inner(self) -> (Self::M, Option<ActorRef>) {
        let Self { envelope, sender } = self;
        (envelope, sender)
    }
}

#[derive(Debug, EmptyCodec)]
struct ClusterEventWrap(ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, Clone, EmptyCodec)]
struct Retry;

#[async_trait]
impl Message for Retry {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}


#[derive(Debug, Clone, EmptyCodec)]
struct RegisterRetry;

#[async_trait]
impl Message for RegisterRetry {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[async_trait]
impl Message for ShardEntityEnvelope {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.deliver_message(context, *self)?;
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
struct Handoff {
    shard: ShardId,
}

#[async_trait]
impl Message for Handoff {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let type_name = &actor.type_name;
        let shard_id = self.shard;
        debug!("{type_name}: Handoff shard [{shard_id}]");
        if actor.shard_buffers.contains_key(&shard_id) {
            let dropped = actor.shard_buffers.drop(
                &shard_id,
                "Avoiding reordering of buffered messages at shard handoff".to_string(),
                context.system().dead_letters(),
            );
            if dropped > 0 {
                let type_name = &actor.type_name;
                warn!("{type_name}: Dropping [{dropped}] buffered messages to shard [{shard_id}] during hand off to avoid re-ordering")
            }
        }
        match actor.shards.get(&shard_id) {
            None => {
                context.sender().foreach(|sender| {
                    sender.cast_ns(ShardStopped { shard: shard_id });
                });
            }
            Some(shard) => {
                actor.handing_off.insert(shard.clone());
                shard.cast(crate::shard::Handoff { shard: shard_id }, context.sender().cloned());
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct ShardTerminated(ActorRef);

impl Terminated for ShardTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for ShardTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let actor_ref = self.0;
        if actor.coordinator.as_ref().is_some_and(|coordinator| coordinator == &actor_ref) {
            actor.coordinator = None;
            actor.start_registration(context)?;
        } else if actor.regions.contains_key(&actor_ref) {
            if let Some(shards) = actor.regions.remove(&actor_ref) {
                for shard in &shards {
                    actor.region_by_shard.remove(shard);
                }
                let type_name = &actor.type_name;
                let size = shards.len();
                let shard_str = shards.into_iter().collect::<Vec<_>>().join(", ");
                debug!("{type_name}: Region [{actor_ref}] terminated with [{size}] shards [{shard_str}]");
            }
        } else if actor.shards_by_ref.contains_key(&actor_ref) {
            if let Some(shard_id) = actor.shards_by_ref.remove(&actor_ref) {
                actor.shards.remove(&shard_id);
                actor.starting_shards.remove(&shard_id);
                //TODO passivation strategy
                let type_name = &actor.type_name;
                match actor.handing_off.remove(&actor_ref) {
                    true => {
                        debug!("{type_name}: Shard [{shard_id}] handoff complete")
                    }
                    false => {
                        debug!("{type_name}: Shard [{shard_id}] terminated while not being handed off");
                    }
                }
            }
        }
        Ok(())
    }
}