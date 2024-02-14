use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::ActorContext;
use actor_core::actor::props::Props;
use actor_core::actor::timers::Timers;
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;

pub type ShardId = String;

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
}

impl Actor for ShardRegion {}
