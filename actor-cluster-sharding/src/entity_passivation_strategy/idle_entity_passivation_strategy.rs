use std::time::Duration;
use crate::entity_passivation_strategy::{PassivateEntities, TEntityPassivationStrategy};
use crate::shard_region::EntityId;

#[derive(Debug)]
pub(crate) struct IdleCheck {
    timeout: Duration,
    interval: Duration,
}

pub(crate) struct IdleEntityPassivationStrategy {
    idle_check: IdleCheck,
}

impl TEntityPassivationStrategy for IdleEntityPassivationStrategy {
    fn limit_updated(&mut self, new_limit: usize) -> PassivateEntities {
        todo!()
    }

    fn shards_updated(&mut self, active_shards: usize) -> PassivateEntities {
        todo!()
    }

    fn entity_touched(&mut self, id: EntityId) -> PassivateEntities {
        todo!()
    }

    fn entity_terminated(&mut self, id: EntityId) -> PassivateEntities {
        todo!()
    }

    fn scheduled_interval(&self) -> Option<Duration> {
        todo!()
    }

    fn interval_passed(&mut self) -> PassivateEntities {
        todo!()
    }
}