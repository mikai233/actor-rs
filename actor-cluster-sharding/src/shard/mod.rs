use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::ops::Not;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use actor_cluster::cluster::Cluster;
use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::message::message_buffer::MessageBufferMap;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard::cluster_event_wrap::ClusterEventWrap;
use crate::shard::entities::Entities;
use crate::shard::entity_state::EntityState;
use crate::shard::entity_terminated::EntityTerminated;
use crate::shard::shard_buffer_envelope::ShardBufferEnvelope;
use crate::shard::shard_envelope::ShardEnvelope;
use crate::shard_region::{EntityId, ShardId};

mod cluster_event_wrap;
mod passivate;
mod entities;
mod handoff;
mod passivate_interval_tick;
mod entity_state;
mod handoff_stopper_terminated;
mod entity_terminated;
mod shard_buffer_envelope;
pub(crate) mod shard_envelope;

#[derive(Debug)]
pub struct Shard {
    type_name: String,
    shard_id: ShardId,
    entity_props: Arc<PropsBuilderSync<EntityId>>,
    settings: Arc<ClusterShardingSettings>,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    entities: Entities,
    message_buffers: MessageBufferMap<EntityId, ShardBufferEnvelope>,
    handoff_stopper: Option<ActorRef>,
    passivate_interval_task: Option<ScheduleKey>,
    preparing_for_shutdown: bool,
    cluster_adapter: ActorRef,
}

impl Shard {
    pub(crate) fn props(
        type_name: String,
        shard_id: ShardId,
        entity_props: Arc<PropsBuilderSync<EntityId>>,
        settings: Arc<ClusterShardingSettings>,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> Props {
        Props::new_with_ctx(move |context| {
            let cluster_adapter = context.message_adapter(|m| DynMessage::user(ClusterEventWrap(m)));
            let shard = Self {
                type_name,
                shard_id,
                entity_props,
                settings,
                extractor,
                handoff_stop_message,
                entities: Default::default(),
                message_buffers: Default::default(),
                handoff_stopper: None,
                passivate_interval_task: None,
                preparing_for_shutdown: false,
                cluster_adapter,
            };
            Ok(shard)
        })
    }

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
        let envelop = ShardBufferEnvelope {
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
        if let Some(messages) = self.message_buffers.remove_buffer(entity_id) {
            if messages.is_empty().not() {
                self.get_or_create_entity(context, entity_id)?;
                debug!("{}: Sending message buffer for entity [{}] ([{}] messages)", self.type_name, entity_id, messages.len());
                for message in messages {
                    let ShardBufferEnvelope { envelope, sender } = message;
                    self.deliver_message(context, envelope, sender)?;
                }
            }
        }
        Ok(())
    }

    fn get_or_create_entity(&mut self, context: &mut ActorContext, entity_id: &EntityId) -> anyhow::Result<ActorRef> {
        match self.entities.entity(entity_id) {
            None => {
                let entity = context.spawn(self.entity_props.props(entity_id.clone()), entity_id)?;
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