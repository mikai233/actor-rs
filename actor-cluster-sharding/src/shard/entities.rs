use std::collections::{HashMap, HashSet};

use eyre::anyhow;
use tracing::debug;

use actor_core::actor_ref::ActorRef;
use actor_core::ext::maybe_ref::MaybeRef;

use crate::shard::entity_state::EntityState;
use crate::shard_region::ImEntityId;

#[derive(Debug, Default)]
pub(super) struct Entities {
    pub(super) entities: HashMap<ImEntityId, EntityState>,
    pub(super) by_ref: HashMap<ActorRef, ImEntityId>,
}

impl Entities {
    pub(super) fn entity_state(&self, entity_id: &str) -> MaybeRef<EntityState> {
        match self.entities.get(entity_id) {
            None => MaybeRef::Own(EntityState::NoState),
            Some(state) => MaybeRef::Ref(state),
        }
    }

    pub(super) fn remove_entity(&mut self, entity_id: &str) -> Option<EntityState> {
        let state = self.entities.remove(entity_id);
        if let Some(state) = &state {
            match state {
                EntityState::Active(_, entity) => {
                    self.by_ref.remove(entity);
                }
                EntityState::Passivation(_, entity) => {
                    self.by_ref.remove(entity);
                }
                _ => {}
            }
        }
        state
    }

    pub(super) fn add_entity(&mut self, entity_id: ImEntityId, entity: ActorRef) {
        self.entities.insert(entity_id.clone(), EntityState::Active(entity_id.clone(), entity.clone()));
        self.by_ref.insert(entity, entity_id);
    }

    pub(super) fn entity(&self, entity_id: &str) -> Option<&ActorRef> {
        match self.entities.get(entity_id) {
            None => None,
            Some(state) => {
                match state {
                    EntityState::Active(_, entity) => Some(entity),
                    EntityState::Passivation(_, entity) => Some(entity),
                    _ => None,
                }
            }
        }
    }

    pub(super) fn entity_id(&self, entity: &ActorRef) -> Option<ImEntityId> {
        self.by_ref.get(entity).cloned()
    }

    pub(super) fn is_passivating(&self, id: &str) -> bool {
        match self.entities.get(id) {
            Some(EntityState::Passivation(_, _)) => true,
            _ => false,
        }
    }

    pub(super) fn entity_passivating(&mut self, entity_id: ImEntityId) -> eyre::Result<()> {
        debug!("[{}] passivating", entity_id);
        match self.entities.remove(&entity_id) {
            None => {
                Err(anyhow!("no entity found with entity id {}", entity_id))
            }
            Some(state) => {
                if let EntityState::Active(entity_id, entity) = state {
                    self.entities.insert(entity_id.clone(), EntityState::Passivation(entity_id, entity));
                    Ok(())
                } else {
                    Err(anyhow!("entity {} not in active state {}", entity_id, state))
                }
            }
        }
    }

    pub(super) fn active_entities(&self) -> HashSet<&ActorRef> {
        self.by_ref.keys().collect()
    }

    pub(super) fn active_entities_num(&self) -> usize {
        self.by_ref.len()
    }

    pub(super) fn active_entity_ids(&self) -> HashSet<&ImEntityId> {
        self.by_ref.values().collect()
    }
}