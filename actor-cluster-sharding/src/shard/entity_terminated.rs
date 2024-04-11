use std::ops::Not;

use async_trait::async_trait;
use tracing::{debug, warn};

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard::entity_state::EntityState;
use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub(super) struct EntityTerminated(pub(super) Terminated);

impl EntityTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for EntityTerminated {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let entity = self.0.actor;
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
                    EntityState::Active(_, _) => {
                        debug!("{}: Entity [{}] terminated", actor.type_name, entity_id);
                        actor.entities.remove_entity(&entity_id);
                    }
                    EntityState::Passivation(_, _) => {
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