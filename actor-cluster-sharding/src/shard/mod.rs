use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::ops::Not;
use std::sync::Arc;

use anyhow::Context as _;
use async_trait::async_trait;
use imstr::ImString;
use tracing::debug;

use actor_cluster::cluster::Cluster;
use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::dead_letter_listener::Dropped;
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::message_buffer::MessageBufferMap;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardEnvelope};
use crate::shard::cluster_event::ClusterEventWrap;
use crate::shard::entities::Entities;
use crate::shard::entity_state::EntityState;
use crate::shard::entity_terminated::EntityTerminated;
use crate::shard::shard_buffer_envelope::ShardBufferEnvelope;
use crate::shard_region::{ImEntityId, ImShardId};
use crate::shard_region::shard_initialized::ShardInitialized;

mod cluster_event;
mod passivate;
mod entities;
pub(crate) mod handoff;
mod passivate_interval_tick;
mod entity_state;
mod handoff_stopper_terminated;
mod entity_terminated;
mod shard_buffer_envelope;
pub(crate) mod shard_envelope;

#[derive(Debug)]
pub struct Shard {
    type_name: ImString,
    shard_id: ImShardId,
    entity_props: PropsBuilder<ImEntityId>,
    settings: Arc<ClusterShardingSettings>,
    extractor: Box<dyn MessageExtractor>,
    handoff_stop_message: DynMessage,
    entities: Entities,
    message_buffers: MessageBufferMap<ImEntityId, ShardBufferEnvelope>,
    handoff_stopper: Option<ActorRef>,
    passivate_interval_task: Option<ScheduleKey>,
    preparing_for_shutdown: bool,//TODO
}

impl Shard {
    pub(crate) fn props(
        type_name: ImString,
        shard_id: ImShardId,
        entity_props: PropsBuilder<ImEntityId>,
        settings: Arc<ClusterShardingSettings>,
        extractor: Box<dyn MessageExtractor>,
        handoff_stop_message: DynMessage,
    ) -> Props {
        Props::new(move || {
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

    fn deliver_message(&mut self, context: &mut ActorContext, message: ShardEnvelope<Shard>, sender: Option<ActorRef>) -> anyhow::Result<()> {
        let message = message.into_shard_region_envelope();
        let entity_id = self.extractor.entity_id(&message);
        let type_name = &self.type_name;
        match &*self.entities.entity_state(&entity_id) {
            EntityState::NoState => {
                let payload = self.extractor.unwrap_message(message);
                let entity_id: ImEntityId = entity_id.into();
                let entity = self.get_or_create_entity(context, &entity_id)?;
                entity.tell(payload, sender);
            }
            EntityState::Active(entity_id, entity) => {
                let payload = self.extractor.unwrap_message(message);
                let message_name = payload.name();
                debug!("{}: Delivering message of type [{}] to [{}]", type_name, message_name, entity_id);
                entity.tell(payload, sender);
            }
            EntityState::Passivation(entity_id, _) => {
                self.append_to_message_buffer(context, entity_id.clone(), message.into_shard_envelope(), sender);
            }
            EntityState::WaitingForRestart => {
                let payload = self.extractor.unwrap_message(message);
                let message_name = payload.name();
                debug!("{}: Delivering message of type [{}] to [{}] (starting because [WaitingForRestart])", type_name, message_name, entity_id);
                let entity_id: ImEntityId = entity_id.into();
                self.get_or_create_entity(context, &entity_id)?.tell(payload, sender);
            }
        }
        Ok(())
    }

    fn append_to_message_buffer(&mut self, context: &mut ActorContext, id: ImEntityId, message: ShardEnvelope<Shard>, sender: Option<ActorRef>) {
        if self.message_buffers.total_size() >= self.settings.buffer_size {
            debug!("{}: Buffer is full, dropping message of type [{}] for entity [{}]", self.type_name, message.message.name(), id);
            let dropped = Dropped::new(message.into_dyn(), format!("Buffer for [{}] is full", id), Some(context.myself().clone()));
            context.system().dead_letters().cast_ns(dropped);
        } else {
            debug!("{}: Message of type [{}] for entity [{}] buffered", self.type_name, message.message.name(), id);
            let envelop = ShardBufferEnvelope {
                message,
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
    }

    fn send_message_buffer(&mut self, context: &mut ActorContext, entity_id: &ImEntityId) -> anyhow::Result<()> {
        if let Some(messages) = self.message_buffers.remove(entity_id) {
            if messages.is_empty().not() {
                self.get_or_create_entity(context, entity_id)?;
                debug!("{}: Sending message buffer for entity [{}] ([{}] messages)", self.type_name, entity_id, messages.len());
                for message in messages {
                    let ShardBufferEnvelope { message: envelope, sender } = message;
                    self.deliver_message(context, envelope, sender)?;
                }
            }
        }
        Ok(())
    }

    fn get_or_create_entity(&mut self, context: &mut ActorContext, entity_id: &ImEntityId) -> anyhow::Result<ActorRef> {
        match self.entities.entity(entity_id) {
            None => {
                let entity = context.spawn(self.entity_props.props(entity_id.clone()), entity_id.as_str())?;
                context.watch(entity.clone(), EntityTerminated::new)?;
                debug!("{}: Started entity [{}] with entity id [{}] in shard [{}]", self.type_name, entity, entity_id, self.shard_id);
                self.entities.add_entity(entity_id.clone(), entity.clone());
                Ok(entity)
            }
            Some(entity) => Ok(entity.clone())
        }
    }


    fn shard_initialized(&self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{}: Shard {} initialized", self.type_name, context.myself());
        context.parent()
            .into_result()
            .context("parent actor not found")
            ?.cast(
            ShardInitialized {
                shard_id: self.shard_id.clone(),
            },
            Some(context.myself().clone()),
        );
        Ok(())
    }
}

#[async_trait]
impl Actor for Shard {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).subscribe_cluster_event(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        self.shard_initialized(context)?;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Cluster::get(context.system()).unsubscribe_cluster_event(context.myself())?;
        if let Some(key) = self.passivate_interval_task.take() {
            key.cancel();
        }
        debug!("{}: Shard [{}] shutting down", self.type_name, self.shard_id);
        Ok(())
    }
}