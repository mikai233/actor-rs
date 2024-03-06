use std::rc::Rc;
use std::time::Duration;

use crate::entity_passivation_strategy::{PassivateEntities, TEntityPassivationStrategy};
use crate::entity_passivation_strategy::recency_list::RecencyList;
use crate::shard_region::EntityId;

#[derive(Debug)]
pub(crate) struct IdleCheck {
    timeout: Duration,
    interval: Duration,
}

pub(crate) struct IdleEntityPassivationStrategy {
    idle_check: IdleCheck,
    recency_list: RecencyList<Rc<EntityId>>,
    scheduled_interval: Duration,
}

impl TEntityPassivationStrategy for IdleEntityPassivationStrategy {
    fn limit_updated(&mut self, _new_limit: usize) -> PassivateEntities {
        vec![]
    }

    fn shards_updated(&mut self, _active_shards: usize) -> PassivateEntities {
        vec![]
    }

    fn entity_touched(&mut self, id: Rc<EntityId>) -> PassivateEntities {
        self.recency_list.update(id);
        vec![]
    }

    fn entity_terminated(&mut self, id: Rc<EntityId>) {
        self.recency_list.remove(&id);
    }

    fn scheduled_interval(&self) -> Option<Duration> {
        Some(self.scheduled_interval)
    }

    fn interval_passed(&mut self) -> PassivateEntities {
        self.recency_list.remove_least_recent_outside(self.idle_check.timeout)
    }
}