use std::time::Duration;
use crate::entity_passivation_strategy::{PassivateEntities, TEntityPassivationStrategy};
use crate::shard_region::EntityId;

pub(crate) struct DisabledEntityPassivationStrategy;

impl TEntityPassivationStrategy for DisabledEntityPassivationStrategy {
    fn limit_updated(&mut self, new_limit: usize) -> PassivateEntities {
        vec![]
    }

    fn shards_updated(&mut self, active_shards: usize) -> PassivateEntities {
        vec![]
    }

    fn entity_touched(&mut self, id: EntityId) -> PassivateEntities {
        vec![]
    }

    fn entity_terminated(&mut self, id: EntityId) -> PassivateEntities {
        vec![]
    }

    fn scheduled_interval(&self) -> Option<Duration> {
        None
    }

    fn interval_passed(&mut self) -> PassivateEntities {
        vec![]
    }
}