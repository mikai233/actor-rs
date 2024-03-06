use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use tracing::debug;

use actor_core::actor_ref::ActorRef;
use actor_core::ext::maybe_ref::MaybeRef;

use crate::shard::entity_state::EntityState;
use crate::shard_region::EntityId;

#[derive(Debug, Default)]
pub(super) struct Entities {
    pub(super) entities: HashMap<EntityId, EntityState>,
    pub(super) by_ref: HashMap<ActorRef, EntityId>,
}

impl Entities {
    pub(super) fn entity_state(&self, entity_id: &EntityId) -> MaybeRef<EntityState> {
        match self.entities.get(entity_id) {
            None => MaybeRef::Own(EntityState::NoState),
            Some(entity) => MaybeRef::Ref(entity)
        }
    }

    pub(super) fn remove_entity(&mut self, entity_id: &EntityId) -> Option<EntityState> {
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

    pub(super) fn add_entity(&mut self, entity_id: EntityId, entity: ActorRef) {
        self.entities.insert(entity_id.clone(), EntityState::Active(entity.clone()));
        self.by_ref.insert(entity, entity_id);
    }

    pub(super) fn entity(&self, entity_id: &EntityId) -> Option<&ActorRef> {
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
    pub(super) fn entity_id(&self, entity: &ActorRef) -> Option<EntityId> {
        self.by_ref.get(entity).cloned()
    }

    pub(super) fn is_passivating(&self, id: &EntityId) -> bool {
        match self.entities.get(id) {
            Some(EntityState::Passivation(_)) => true,
            _ => false,
        }
    }

    pub(super) fn entity_passivating(&mut self, entity_id: EntityId) -> anyhow::Result<()> {
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

    pub(super) fn active_entities(&self) -> HashSet<&ActorRef> {
        self.by_ref.keys().collect()
    }

    pub(super) fn active_entities_num(&self) -> usize {
        self.by_ref.len()
    }

    pub(super) fn active_entity_ids(&self) -> HashSet<&EntityId> {
        self.by_ref.values().collect()
    }
}