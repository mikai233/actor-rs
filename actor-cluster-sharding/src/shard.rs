use std::collections::{HashMap, HashSet};
use std::ops::Not;
use async_trait::async_trait;
use dashmap::mapref::one::MappedRef;
use tracing::{debug, warn};
use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_event::ClusterEvent;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::ext::maybe_ref::MaybeRef;
use actor_core::message::message_buffer::MessageBufferMap;
use actor_derive::EmptyCodec;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_region::{EntityId, ShardId};

#[derive(Debug)]
pub struct Shard {
    type_name: String,
    shard_id: ShardId,
    entity_props: Props,
    settings: ClusterShardingSettings,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    entities: Entities,
    message_buffers: MessageBufferMap<EntityId>,
    handoff_stopper: Option<ActorRef>,
    preparing_for_shutdown: bool,
    cluster_adapter: ActorRef,
}

impl Shard {}

#[async_trait]
impl Actor for Shard {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).subscribe_cluster_event(self.cluster_adapter.clone());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).unsubscribe_cluster_event(&self.cluster_adapter);
        Ok(())
    }
}

#[derive(Debug)]
enum EntityState {
    NoState,
    Active(ActorRef),
    Passivation(ActorRef),
    WaitingForRestart,
}

#[derive(Debug)]
struct Entities {
    entities: HashMap<EntityId, EntityState>,
    by_ref: HashMap<ActorRef, EntityId>,
}

impl Entities {
    fn entity_state(&self, entity_id: &EntityId) -> MaybeRef<EntityState> {
        match self.entities.get(entity_id) {
            None => MaybeRef::Own(EntityState::NoState),
            Some(entity) => MaybeRef::Ref(entity)
        }
    }

    fn remove_entity(&mut self, entity_id: &EntityId) -> Option<EntityState> {
        let state = self.entities.remove(entity_id);
        if let Some(state) = &state {
            match state {
                EntityState::Active(entity) => {
                    self.by_ref.remove(entity);
                }
                EntityState::Passivation(entity) => {
                    self.by_ref.remove(entity);
                }
                _ => {}
            }
        }
        state
    }

    fn add_entity(&mut self, entity_id: EntityId, entity: ActorRef) {
        self.entities.insert(entity_id.clone(), EntityState::Active(entity.clone()));
        self.by_ref.insert(entity, entity_id);
    }

    fn entity_id(&self, entity: &ActorRef) -> Option<&EntityId> {
        self.by_ref.get(entity)
    }

    fn is_passivating(&self, id: &EntityId) -> bool {
        match self.entities.get(id) {
            Some(EntityState::Passivation(_)) => true,
            _ => false,
        }
    }

    fn entity_passivating(&mut self, entity_id: &EntityId) -> anyhow::Result<()> {
        debug!("[{}] passivating", entity_id);
        Ok(())
    }

    fn active_entities(&self) -> HashSet<&ActorRef> {
        self.by_ref.keys().collect()
    }

    fn active_entities_num(&self) -> usize {
        self.by_ref.len()
    }

    fn active_entity_ids(&self) -> HashSet<&EntityId> {
        self.by_ref.values().collect()
    }
}

#[derive(Debug, EmptyCodec)]
struct ClusterEventWrap(ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, EmptyCodec)]
pub struct Passivate {
    pub stop_message: DynMessage,
}

#[async_trait]
impl Message for Passivate {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let type_name = &actor.type_name;
        match context.sender() {
            None => {
                let stop_message_name = self.stop_message.name();
                warn!("ignore Passivate:{} message for {} because Passivate sender is none", stop_message_name, type_name);
            }
            Some(entity) => {
                match actor.entities.entity_id(entity) {
                    None => {
                        debug!("{}: Unknown entity passivating [{}]. Not send stop message back to entity", type_name, entity);
                    }
                    Some(id) => {
                        if actor.entities.is_passivating(id) {
                            debug!("{}: Passivation already in progress for [{}]. Not sending stop message back to entity", type_name, id);
                        } else if actor.message_buffers.get(id).is_some_and(|buffer| { buffer.is_empty().not() }) {
                            debug!("{}: Passivation when there are buffered messages for [{}], ignoring passivation", type_name, id);
                        } else {}
                    }
                }
            }
        }
        Ok(())
    }
}