use std::time::Duration;

use ahash::HashSet;
use async_trait::async_trait;
use imstr::ImString;

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::actor_ref::ActorRef;

use crate::handoff_stopper::entity_terminated::EntityTerminated;
use crate::handoff_stopper::stop_timeout::StopTimeout;
use crate::handoff_stopper::stop_timeout_warning::StopTimeoutWarning;
use crate::shard_region::ImShardId;

mod entity_terminated;
mod stop_timeout_warning;
mod stop_timeout;

const STOP_TIMEOUT_WARNING_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct HandoffStopper {
    type_name: ImString,
    shard: ImShardId,
    replay_to: ActorRef,
    remaining_entities: HashSet<ActorRef>,
    stop_message: DynMessage,
    entity_handoff_timeout: Duration,
    timers: Timers,
    stop_timeout_warning_key: Option<ScheduleKey>,
    stop_timeout_key: Option<ScheduleKey>,
}

impl HandoffStopper {
    pub(crate) fn props(
        type_name: ImString,
        shard: ImShardId,
        replay_to: ActorRef,
        entities: HashSet<ActorRef>,
        stop_message: DynMessage,
        handoff_timeout: Duration,
    ) -> Props {
        Props::new_with_ctx(move |context| {
            Self::new(context, type_name, shard, replay_to, entities, stop_message, handoff_timeout)
        })
    }

    fn new(
        context: &mut ActorContext,
        type_name: ImString,
        shard: ImShardId,
        replay_to: ActorRef,
        entities: HashSet<ActorRef>,
        stop_message: DynMessage,
        handoff_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let entity_handoff_timeout = (handoff_timeout - STOP_TIMEOUT_WARNING_AFTER).max(Duration::from_secs(1));
        let timers = Timers::new(context)?;
        let stop_timeout_warning_key = timers.start_single_timer(
            STOP_TIMEOUT_WARNING_AFTER,
            StopTimeoutWarning,
            context.myself().clone(),
        );
        let stop_timeout_key = timers.start_single_timer(
            entity_handoff_timeout,
            StopTimeout,
            context.myself().clone(),
        );
        for entity in &entities {
            context.watch(entity.clone(), EntityTerminated::new)?;
            entity.tell(stop_message.dyn_clone()?, ActorRef::no_sender());
        }
        let stopper = Self {
            type_name,
            shard,
            replay_to,
            remaining_entities: Default::default(),
            stop_message,
            entity_handoff_timeout,
            timers,
            stop_timeout_warning_key: Some(stop_timeout_warning_key),
            stop_timeout_key: Some(stop_timeout_key),
        };
        Ok(stopper)
    }
}

#[async_trait]
impl Actor for HandoffStopper {
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}