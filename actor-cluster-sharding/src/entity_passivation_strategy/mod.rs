mod disabled_entity_passivation_strategy;
mod idle_entity_passivation_strategy;
mod recency_list;

use std::rc::Rc;
use std::time::Duration;
use enum_dispatch::enum_dispatch;
use crate::entity_passivation_strategy::disabled_entity_passivation_strategy::DisabledEntityPassivationStrategy;
use crate::shard_region::EntityId;

pub(crate) type PassivateEntities = Vec<Rc<EntityId>>;

#[enum_dispatch]
pub(crate) enum EntityPassivationStrategy {
    DisabledEntityPassivationStrategy,
}


#[enum_dispatch(EntityPassivationStrategy)]
pub(crate) trait TEntityPassivationStrategy {
    fn limit_updated(&mut self, new_limit: usize) -> PassivateEntities;

    fn shards_updated(&mut self, active_shards: usize) -> PassivateEntities;

    fn entity_touched(&mut self, id: Rc<EntityId>) -> PassivateEntities;

    fn entity_terminated(&mut self, id: Rc<EntityId>);

    fn scheduled_interval(&self) -> Option<Duration>;

    fn interval_passed(&mut self) -> PassivateEntities;
}