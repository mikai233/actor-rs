use std::collections::{HashMap, HashSet, VecDeque};
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::ops::Not;

use anyhow::anyhow;
use async_trait::async_trait;
use tracing::{debug, warn};

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_event::ClusterEvent;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::ext::maybe_ref::MaybeRef;
use actor_core::message::message_buffer::MessageBufferMap;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardingEnvelope};
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
    message_buffers: MessageBufferMap<EntityId, BufferEnvelope>,
    handoff_stopper: Option<ActorRef>,
    passivate_interval_task: Option<ScheduleKey>,
    preparing_for_shutdown: bool,
    cluster_adapter: ActorRef,
}

impl Shard {
    fn passivate(&mut self, entity: &ActorRef, stop_message: DynMessage) -> anyhow::Result<()> {
        let type_name = &self.type_name;
        match self.entities.entity_id(&entity) {
            None => {
                debug!("{}: Unknown entity passivating [{}]. Not send stop message back to entity", type_name, entity);
            }
            Some(id) => {
                if self.entities.is_passivating(&id) {
                    debug!("{}: Passivation already in progress for [{}]. Not sending stop message back to entity", type_name, id);
                } else if self.message_buffers.get(&id).is_some_and(|buffer| { buffer.is_empty().not() }) {
                    debug!("{}: Passivation when there are buffered messages for [{}], ignoring passivation", type_name, id);
                } else {
                    self.entities.entity_passivating(id)?;
                    entity.tell(stop_message, ActorRef::no_sender());
                }
            }
        }
        Ok(())
    }

    fn deliver_message(&mut self, context: &mut ActorContext, message: ShardEnvelope, sender: Option<ActorRef>) -> anyhow::Result<()> {
        let entity_id = self.extractor.entity_id(&message.0);
        let type_name = &self.type_name;
        match &*self.entities.entity_state(&entity_id) {
            EntityState::NoState => {
                let payload = self.extractor.unwrap_message(message.0);
                let entity = self.get_or_create_entity(context, &entity_id)?;
                entity.tell(payload, sender);
            }
            EntityState::Active(entity) => {
                let payload = self.extractor.unwrap_message(message.0);
                let message_name = payload.name;
                debug!("{}: Delivering message of type [{}] to [{}]", type_name, message_name, entity_id);
                entity.tell(payload, sender);
            }
            EntityState::Passivation(_) => {
                self.append_to_message_buffer(entity_id.clone(), message, sender);
            }
            EntityState::WaitingForRestart => {
                let payload = self.extractor.unwrap_message(message.0);
                let message_name = payload.name;
                debug!("{}: Delivering message of type [{}] to [{}] (starting because [WaitingForRestart])", type_name, message_name, entity_id);
                self.get_or_create_entity(context, &entity_id)?.tell(payload, sender);
            }
        }
        Ok(())
    }

    fn append_to_message_buffer(&mut self, id: EntityId, message: ShardEnvelope, sender: Option<ActorRef>) {
        // TODO buffer size overflow
        let envelop = BufferEnvelope {
            envelope: message,
            sender,
        };
        match self.message_buffers.entry(id) {
            Entry::Occupied(mut o) => {
                o.get_mut().push_back(envelop);
            }
            Entry::Vacant(v) => {
                let mut buffer = VecDeque::new();
                buffer.push_back(envelop);
                v.insert(buffer);
            }
        }
    }

    fn send_message_buffer(&mut self, context: &mut ActorContext, entity_id: &EntityId) -> anyhow::Result<()> {
        if let Some(messages) = self.message_buffers.remove(entity_id) {
            if messages.is_empty().not() {
                self.get_or_create_entity(context, entity_id)?;
                debug!("{}: Sending message buffer for entity [{}] ([{}] messages)", self.type_name, entity_id, messages.len());
                for message in messages {
                    let BufferEnvelope { envelope, sender } = message;
                    self.deliver_message(context, envelope, sender)?;
                }
            }
        }
        Ok(())
    }

    fn get_or_create_entity(&mut self, context: &mut ActorContext, entity_id: &EntityId) -> anyhow::Result<ActorRef> {
        match self.entities.entity(entity_id) {
            None => {
                let entity = context.spawn(self.entity_props.clone(), entity_id)?;
                context.watch(EntityTerminated(entity.clone()));
                debug!("{}: Started entity [{}] with entity id [{}] in shard [{}]", self.type_name, entity, entity_id, self.shard_id);
                self.entities.add_entity(entity_id.clone(), entity.clone());
                Ok(entity)
            }
            Some(entity) => Ok(entity.clone())
        }
    }
}

#[async_trait]
impl Actor for Shard {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).subscribe_cluster_event(self.cluster_adapter.clone());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).unsubscribe_cluster_event(&self.cluster_adapter);
        if let Some(key) = self.passivate_interval_task.take() {
            key.cancel();
        }
        debug!("{}: Shard [{}] shutting down", self.type_name, self.shard_id);
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

impl Display for EntityState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityState::NoState => {
                write!(f, "NoState")
            }
            EntityState::Active(_) => {
                write!(f, "Active")
            }
            EntityState::Passivation(_) => {
                write!(f, "Passivation")
            }
            EntityState::WaitingForRestart => {
                write!(f, "WaitingForRestart")
            }
        }
    }
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

    fn entity(&self, entity_id: &EntityId) -> Option<&ActorRef> {
        match self.entities.get(entity_id) {
            None => None,
            Some(state) => {
                match state {
                    EntityState::Active(entity) => Some(entity),
                    EntityState::Passivation(entity) => Some(entity),
                    _ => None,
                }
            }
        }
    }

    /// 最好的方式是这里直接返回[EntityId]的引用，但是目前rust不支持从self中部分借用结构体中的数据
    /// 只能整个借用self，这样会导致拿到[EntityId]的引用再去调用结构体的可变方法时会发生借用冲突
    /// 这里只能先直接克隆了，或者另外一种方式是不要封装成方法
    fn entity_id(&self, entity: &ActorRef) -> Option<EntityId> {
        self.by_ref.get(entity).cloned()
    }

    fn is_passivating(&self, id: &EntityId) -> bool {
        match self.entities.get(id) {
            Some(EntityState::Passivation(_)) => true,
            _ => false,
        }
    }

    fn entity_passivating(&mut self, entity_id: EntityId) -> anyhow::Result<()> {
        debug!("[{}] passivating", entity_id);
        match self.entities.remove(&entity_id) {
            None => {
                Err(anyhow!("no entity found with entity id {}", entity_id))
            }
            Some(state) => {
                if let EntityState::Active(entity) = state {
                    self.entities.insert(entity_id, EntityState::Passivation(entity));
                    Ok(())
                } else {
                    Err(anyhow!("entity {} not in active state {}", entity_id, state))
                }
            }
        }
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
        match context.sender() {
            None => {
                let name = self.stop_message.name();
                let type_name = &actor.type_name;
                warn!("Ignore Passivate:{} message for {} because Passivate sender is none", name, type_name);
            }
            Some(entity) => {
                actor.passivate(entity, self.stop_message)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct ShardEnvelope(ShardingEnvelope);

#[async_trait]
impl Message for ShardEnvelope {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug)]
struct BufferEnvelope {
    envelope: ShardEnvelope,
    sender: Option<ActorRef>,
}

#[derive(Debug, EmptyCodec)]
struct EntityTerminated(ActorRef);

impl Terminated for EntityTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for EntityTerminated {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let entity = self.0;
        match actor.entities.entity_id(&entity) {
            None => {
                warn!("{}: Unexpected entity terminated: {}", actor.type_name, entity);
            }
            Some(entity_id) => {
                // TODO passivationStrategy
                match &*actor.entities.entity_state(&entity_id) {
                    EntityState::NoState => {
                        debug!("{}: Got a terminated for [{}], entity id [{}] which is in unexpected state NoState", actor.type_name, entity, entity_id);
                    }
                    EntityState::Active(_) => {
                        debug!("{}: Entity [{}] terminated", actor.type_name, entity_id);
                        actor.entities.remove_entity(&entity_id);
                    }
                    EntityState::Passivation(_) => {
                        if let Some(messages) = actor.message_buffers.remove(&entity_id) {
                            if messages.is_empty().not() {
                                debug!("{}: [{}] terminated after passivating, buffered messages found, restarting", actor.type_name, entity_id);
                                actor.entities.remove_entity(&entity_id);
                                actor.get_or_create_entity(context, &entity_id)?;
                                actor.send_message_buffer(context, &entity_id)?;
                            } else {
                                debug!("{}: [{}] terminated after passivating", actor.type_name, entity_id);
                                actor.entities.remove_entity(&entity_id);
                            }
                        } else {
                            debug!("{}: [{}] terminated after passivating", actor.type_name, entity_id);
                            actor.entities.remove_entity(&entity_id);
                        }
                    }
                    EntityState::WaitingForRestart => {
                        debug!("{}: Got a terminated for [{}], entity id [{}] which is in unexpected state NoState", actor.type_name, entity, entity_id);
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct HandoffStopperTerminated(ActorRef);

#[async_trait]
impl Message for HandoffStopperTerminated {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.handoff_stopper.as_ref().is_some_and(|a| a == &self.0) {
            context.stop(context.myself());
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct PassivateIntervalTick;

#[async_trait]
impl Message for PassivateIntervalTick {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}