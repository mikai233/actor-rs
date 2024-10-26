use std::ops::Not;

use crate::shard::entity_state::EntityState;
use crate::shard::Shard;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::message::terminated::Terminated;
use actor_core::Message;
use tracing::{debug, warn};

#[derive(Debug, Message, derive_more::Display)]
#[display("EntityTerminated({_0})")]
pub(super) struct EntityTerminated(pub(super) ActorRef);

impl MessageHandler<Shard> for EntityTerminated {
    fn handle(actor: &mut Shard, ctx: &mut <Shard as Actor>::Context, message: Self, _: Option<ActorRef>, _: &Receive<Shard>) -> anyhow::Result<Behavior<Shard>> {
        let entity = message.0;
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
                                actor.get_or_create_entity(ctx, &entity_id)?;
                                actor.send_message_buffer(ctx, &entity_id)?;
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
        Ok(Behavior::same())
    }
}