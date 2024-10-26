use anyhow::anyhow;
use imstr::ImString;
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::ops::Not;
use std::sync::Arc;
use tracing::debug;

use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::{MessageExtractor, ShardEnvelope};
use crate::shard::cluster_event::ClusterEventWrap;
use crate::shard::entities::Entities;
use crate::shard::entity_state::EntityState;
use crate::shard::entity_terminated::EntityTerminated;
use crate::shard::shard_buffer_envelope::ShardBufferEnvelope;
use crate::shard_region::shard_initialized::ShardInitialized;
use crate::shard_region::{ImEntityId, ImShardId};
use actor_cluster::cluster::Cluster;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::dead_letter_listener::Dropped;
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor::receive::Receive;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::message_buffer::MessageBufferMap;
use actor_core::message::terminated::Terminated;
use actor_core::message::DynMessage;

mod cluster_event;
mod entities;
mod entity_state;
pub(crate) mod handoff;
pub mod passivate;
mod passivate_interval_tick;
mod shard_buffer_envelope;
pub(crate) mod shard_envelope;
mod entity_terminated;

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
    preparing_for_shutdown: bool, //TODO
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
                debug!(
                    "{}: Unknown entity passivating [{}]. Not send stop message back to entity",
                    type_name, entity
                );
            }
            Some(id) => {
                if self.entities.is_passivating(&id) {
                    debug!("{}: Passivation already in progress for [{}]. Not sending stop message back to entity", type_name, id);
                } else if self
                    .message_buffers
                    .get(&id)
                    .is_some_and(|buffer| buffer.is_empty().not())
                {
                    debug!("{}: Passivation when there are buffered messages for [{}], ignoring passivation", type_name, id);
                } else {
                    self.entities.entity_passivating(id)?;
                    entity.cast_ns(stop_message);
                }
            }
        }
        Ok(())
    }

    fn deliver_message(
        &mut self,
        ctx: &mut <Shard as Actor>::Context,
        message: ShardEnvelope,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<()> {
        let entity_id = self.extractor.entity_id(&message);
        let type_name = &self.type_name;
        match &*self.entities.entity_state(&entity_id) {
            EntityState::NoState => {
                let payload = self.extractor.unwrap_message(message);
                let entity_id: ImEntityId = entity_id.into();
                let entity = self.get_or_create_entity(ctx, &entity_id)?;
                entity.tell(payload, sender);
            }
            EntityState::Active(entity_id, entity) => {
                let payload = self.extractor.unwrap_message(message);
                let message_name = payload.name();
                debug!(
                    "{}: Delivering message of type [{}] to [{}]",
                    type_name, message_name, entity_id
                );
                entity.tell(payload, sender);
            }
            EntityState::Passivation(entity_id, _) => {
                self.append_to_message_buffer(
                    ctx,
                    entity_id.clone(),
                    message,
                    sender,
                );
            }
            EntityState::WaitingForRestart => {
                let payload = self.extractor.unwrap_message(message);
                let message_name = payload.name();
                debug!("{}: Delivering message of type [{}] to [{}] (starting because [WaitingForRestart])", type_name, message_name, entity_id);
                let entity_id: ImEntityId = entity_id.into();
                self.get_or_create_entity(ctx, &entity_id)?
                    .tell(payload, sender);
            }
        }
        Ok(())
    }

    fn append_to_message_buffer(
        &mut self,
        ctx: &mut <Shard as Actor>::Context,
        id: ImEntityId,
        message: ShardEnvelope,
        sender: Option<ActorRef>,
    ) {
        if self.message_buffers.total_size() >= self.settings.buffer_size {
            debug!(
                "{}: Buffer is full, dropping message of type [{}] for entity [{}]",
                self.type_name,
                message.message.name(),
                id
            );
            let dropped = Dropped::new(
                Box::new(message),
                format!("Buffer for [{}] is full", id),
                Some(ctx.myself().clone()),
            );
            ctx.system().dead_letters().cast_ns(dropped);
        } else {
            debug!(
                "{}: Message of type [{}] for entity [{}] buffered",
                self.type_name,
                message.message.name(),
                id
            );
            let envelop = ShardBufferEnvelope { message, sender };
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

    fn send_message_buffer(
        &mut self,
        ctx: &mut <Shard as Actor>::Context,
        entity_id: &ImEntityId,
    ) -> anyhow::Result<()> {
        if let Some(messages) = self.message_buffers.remove(entity_id) {
            if messages.is_empty().not() {
                self.get_or_create_entity(ctx, entity_id)?;
                debug!(
                    "{}: Sending message buffer for entity [{}] ([{}] messages)",
                    self.type_name,
                    entity_id,
                    messages.len()
                );
                for message in messages {
                    let ShardBufferEnvelope {
                        message: envelope,
                        sender,
                    } = message;
                    self.deliver_message(ctx, envelope, sender)?;
                }
            }
        }
        Ok(())
    }

    fn get_or_create_entity(
        &mut self,
        ctx: &mut <Shard as Actor>::Context,
        entity_id: &ImEntityId,
    ) -> anyhow::Result<ActorRef> {
        match self.entities.entity(entity_id) {
            None => {
                let entity = ctx.spawn(
                    self.entity_props.props(entity_id.clone()),
                    entity_id.as_str(),
                )?;
                ctx.watch_with(&entity, Box::new(EntityTerminated(entity.clone())))?;
                debug!(
                    "{}: Started entity [{}] with entity id [{}] in shard [{}]",
                    self.type_name, entity, entity_id, self.shard_id
                );
                self.entities.add_entity(entity_id.clone(), entity.clone());
                Ok(entity)
            }
            Some(entity) => Ok(entity.clone()),
        }
    }

    fn shard_initialized(&self, ctx: &mut <Shard as Actor>::Context) -> anyhow::Result<()> {
        debug!("{}: Shard {} initialized", self.type_name, ctx.myself());
        let parent = ctx.parent().ok_or(anyhow!("Parent not found"))?;
        parent.cast(ShardInitialized::new(self.shard_id.clone()), Some(ctx.myself().clone()));
        Ok(())
    }

    fn receive_terminated(&mut self, ctx: &mut <Shard as Actor>::Context, actor_ref: ActorRef) -> anyhow::Result<()> {
        if self
            .handoff_stopper
            .as_ref()
            .is_some_and(|a| a == &actor_ref)
        {
            ctx.stop(ctx.myself());
        }
        Ok(())
    }
}

impl Actor for Shard {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        Cluster::get(ctx.system()).subscribe(ctx.myself().clone(), |event| {
            ClusterEventWrap(event).into_dyn()
        })?;
        self.shard_initialized(ctx)?;
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        Cluster::get(ctx.system()).unsubscribe_cluster_event(ctx.myself())?;
        if let Some(key) = self.passivate_interval_task.take() {
            key.cancel();
        }
        debug!(
            "{}: Shard [{}] shutting down",
            self.type_name, self.shard_id
        );
        Ok(())
    }

    fn receive(&self) -> Receive<Self> {
        Receive::new()
            .is::<Terminated>(|actor, ctx, t, _, _| {
                actor.receive_terminated(ctx, t.actor_ref)?;
                Ok(Behavior::same())
            })
    }
}
