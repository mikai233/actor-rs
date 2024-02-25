use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tracing::warn;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::CoordinatedShutdown;
use actor_core::actor::props::Props;
use actor_core::actor::timers::{ScheduleKey, Timers};
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::ShardStopped;
use crate::shard_region::ShardId;

const STOP_TIMEOUT_WARNING_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct HandoffStopper {
    type_name: String,
    shard: ShardId,
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
        type_name: String,
        shard: ShardId,
        replay_to: ActorRef,
        entities: HashSet<ActorRef>,
        stop_message: DynMessage,
        handoff_timeout: Duration,
    ) -> Props {
        let stop_message = Arc::new(Mutex::new(stop_message));
        Props::create(move |context| {
            let stop_message = stop_message.lock().unwrap().dyn_clone().into_result()?;
            Self::new(context, type_name.clone(), shard.clone(), replay_to.clone(), entities.clone(), stop_message, handoff_timeout)
        })
    }

    fn new(
        context: &mut ActorContext,
        type_name: String,
        shard: String,
        replay_to: ActorRef,
        entities: HashSet<ActorRef>,
        stop_message: DynMessage,
        handoff_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let entity_handoff_timeout = (handoff_timeout - STOP_TIMEOUT_WARNING_AFTER).max(Duration::from_secs(1));
        let timers = Timers::new(context)?;
        let stop_timeout_warning_key = timers.start_single_timer(
            STOP_TIMEOUT_WARNING_AFTER,
            DynMessage::user(StopTimeoutWarning),
            context.myself().clone(),
        );
        let stop_timeout_key = timers.start_single_timer(
            entity_handoff_timeout,
            DynMessage::user(StopTimeout),
            context.myself().clone(),
        );
        for entity in &entities {
            let entity_terminated = EntityTerminated(entity.clone());
            context.watch(entity_terminated);
            entity.tell(stop_message.dyn_clone().into_result()?, ActorRef::no_sender());
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
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct StopTimeoutWarning;

#[async_trait]
impl Message for StopTimeoutWarning {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let is_terminating = CoordinatedShutdown::get(context.system()).is_terminating();
        let type_name = &actor.type_name;
        let remaining_size = actor.remaining_entities.len();
        let shard = &actor.shard;
        let timeout = &STOP_TIMEOUT_WARNING_AFTER;
        let stop_msg = actor.stop_message.name;
        if is_terminating {
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled?");
        } else {
            let remaining_timeout = actor.entity_handoff_timeout - *timeout;
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled? \
            Waiting additional [{remaining_timeout:?}] before stopping the remaining entities.");
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct StopTimeout;

#[async_trait]
impl Message for StopTimeout {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let type_name = &actor.type_name;
        let stop_msg = actor.stop_message.name;
        let shard = &actor.shard;
        let timeout = &actor.entity_handoff_timeout;
        let remaining_size = actor.remaining_entities.len();
        warn!("{type_name}: handoff stop message [{stop_msg}] is not handled by some of the entities in shard [{shard}] after [{timeout:?}], stopping the remaining [{remaining_size}] entities");
        if let Some(key) = actor.stop_timeout_warning_key.take() {
            key.cancel();
        }
        for entity in &actor.remaining_entities {
            context.stop(entity);
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct EntityTerminated(ActorRef);

impl Terminated for EntityTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for EntityTerminated {
    type A = HandoffStopper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let entity = self.0;
        actor.remaining_entities.remove(&entity);
        if actor.remaining_entities.is_empty() {
            actor.replay_to.cast_ns(ShardStopped { shard: actor.shard.clone() });
            context.stop(context.myself());
        }
        Ok(())
    }
}