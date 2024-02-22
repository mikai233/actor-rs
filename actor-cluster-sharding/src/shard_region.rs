use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Mul, Not};
use std::sync::Mutex;
use std::time::Duration;
use async_trait::async_trait;
use tracing::{debug, warn};
use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_event::ClusterEvent;
use actor_cluster::member::{Member, MemberStatus};
use actor_cluster::unique_address::UniqueAddress;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::message::poison_pill::PoisonPill;
use actor_derive::EmptyCodec;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardingEnvelope};
use crate::shard_coordinator::{Register, RegisterProxy};

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
    shard_buffers: HashMap<ShardId, VecDeque<(DynMessage, Option<ActorRef>)>>,
    shards: HashMap<ShardId, ActorRef>,
    shards_by_ref: HashMap<ActorRef, ShardId>,
    starting_shards: HashSet<ShardId>,
    retry_count: usize,
    init_registration_delay: Duration,
    next_registration_delay: Duration,
    cluster: Cluster,
    cluster_event_adapter: ActorRef,
    register_retry_key: Option<ScheduleKey>,
    members: HashMap<UniqueAddress, Member>,
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
            retry_count: 0,
            init_registration_delay: Duration::from_secs(1),
            next_registration_delay: Duration::from_secs(1),
            cluster,
            cluster_event_adapter: context.message_adapter(|m| { DynMessage::user(ClusterEventWrap(m)) }),
            register_retry_key: None,
            members: Default::default(),
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
                let buffer_size = self.shard_buffer_total_size();
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

    fn shard_buffer_total_size(&self) -> usize {
        self.shard_buffers.values().fold(0, |acc, buffer| { acc + buffer.len() })
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
impl Message for ShardingEnvelope {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}