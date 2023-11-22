use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use futures::task::ArcWake;
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;
use tracing::{debug, warn};

use actor_derive::EmptyCodec;

use crate::{Actor, AsyncMessage, DynamicMessage, Message};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::context::{ActorContext, Context};
use crate::system::message_factory::MessageFactory;

pub(crate) struct TimerScheduler;

pub(crate) struct State {
    queue: DelayQueue<Schedule>,
    key: HashMap<u128, Key>,
    waker: futures::task::Waker,
}

#[derive(Debug, Clone)]
struct ScheduleKey {
    key: u128,
    scheduler: ActorRef,
}

impl ScheduleKey {
    fn cancel(self) {
        self.scheduler.cast(CancelSchedule { key: self.key }, ActorRef::no_sender());
    }
}

#[derive(EmptyCodec)]
enum Schedule {
    Once {
        key: u128,
        delay: Duration,
        message: DynamicMessage,
        receiver: ActorRef,
    },
    FixedRate {
        key: u128,
        initial_delay: Option<Duration>,
        delay: Duration,
        factory: Box<dyn MessageFactory>,
        receiver: ActorRef,
    },
}

impl Message for Schedule {
    type T = TimerScheduler;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        match &*self {
            Schedule::Once { key, delay, .. } => {
                let key = *key;
                let delay = delay.clone();
                let delay_key = state.queue.insert(*self, delay);
                debug_assert!(!state.key.contains_key(&key));
                state.key.insert(key, delay_key);
            }
            Schedule::FixedRate { key, initial_delay, delay, .. } => {
                let key = *key;
                let delay_key = match initial_delay {
                    None => {
                        let delay = delay.clone();
                        state.queue.insert(*self, delay)
                    }
                    Some(initial_delay) => {
                        let delay = initial_delay.clone();
                        state.queue.insert(*self, delay)
                    }
                };
                debug_assert!(!state.key.contains_key(&key));
                state.key.insert(key, delay_key);
            }
        }
        context.myself().cast(PollExpired, ActorRef::no_sender());
        Ok(())
    }
}

impl Debug for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Schedule::Once { key, delay, message, receiver } => {
                f.debug_struct("Once")
                    .field("key", key)
                    .field("delay", delay)
                    .field("message", message)
                    .field("receiver", receiver)
                    .finish()
            }
            Schedule::FixedRate { key, initial_delay, delay, factory: message, receiver } => {
                f.debug_struct("FixedRate")
                    .field("key", key)
                    .field("initial_delay", initial_delay)
                    .field("delay", delay)
                    .field("message", &"..")
                    .field("receiver", receiver)
                    .finish()
            }
        }
    }
}

impl Actor for TimerScheduler {
    type S = State;
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        let myself = context.myself();
        debug!("{} pre start", myself);
        let waker = futures::task::waker(Arc::new(SchedulerWaker { scheduler: myself.clone() }));
        let state = State {
            queue: DelayQueue::new(),
            key: HashMap::new(),
            waker,
        };
        Ok(state)
    }
}

#[derive(Debug, EmptyCodec)]
struct CancelSchedule {
    key: u128,
}

impl Message for CancelSchedule {
    type T = TimerScheduler;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        match state.key.remove(&self.key) {
            None => {
                warn!("{} not found in TimerScheduler", self.key);
            }
            Some(key) => {
                state.queue.try_remove(&key);
                debug!("{} removed from TimerScheduler", self.key);
            }
        }
        context.myself().cast(PollExpired, ActorRef::no_sender());
        Ok(())
    }
}

#[derive(EmptyCodec)]
struct PollExpired;

impl Message for PollExpired {
    type T = TimerScheduler;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let waker = &state.waker;
        let queue = &mut state.queue;
        let key_map = &mut state.key;
        let mut context = futures::task::Context::from_waker(waker);

        while let Poll::Ready(Some(expired)) = queue.poll_expired(&mut context) {
            let schedule = expired.into_inner();
            match schedule {
                Schedule::Once { key, message, receiver, .. } => {
                    key_map.remove(&key);
                    receiver.tell(message, ActorRef::no_sender());
                }
                Schedule::FixedRate { key, delay, factory, receiver, .. } => {
                    let message = factory.message();
                    receiver.tell(message, ActorRef::no_sender());
                    let next_delay = delay;
                    let reschedule = Schedule::FixedRate {
                        key,
                        initial_delay: None,
                        delay,
                        factory,
                        receiver,
                    };
                    let next_key = queue.insert(reschedule, next_delay);
                    key_map.insert(key, next_key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct SchedulerWaker {
    scheduler: ActorRef,
}

impl ArcWake for SchedulerWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.scheduler.cast(PollExpired, ActorRef::no_sender());
    }
}

#[cfg(test)]
mod scheduler_test {
    use std::time::Duration;

    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, DynamicMessage, EmptyTestActor, Message};
    use crate::actor_ref::{ActorRef, ActorRefExt};
    use crate::context::{ActorContext, Context};
    use crate::message::MessageRegistration;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;
    use crate::system::message_factory::MessageFactory;
    use crate::system::timer_scheduler::{Schedule, TimerScheduler};

    #[derive(Debug, EmptyCodec)]
    struct OnOnceSchedule;

    impl Message for OnOnceSchedule {
        type T = EmptyTestActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            info!("{} OnOnceSchedule", context.myself());
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct OnFixedSchedule;

    impl Message for OnFixedSchedule {
        type T = EmptyTestActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            info!("{} OnFixedSchedule", context.myself());
            Ok(())
        }
    }

    struct FixedScheduleFactory;

    impl MessageFactory for FixedScheduleFactory {
        fn message(&self) -> DynamicMessage {
            DynamicMessage::user(OnFixedSchedule)
        }
    }


    #[tokio::test]
    async fn test_scheduler() -> anyhow::Result<()> {
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse().unwrap(), MessageRegistration::new())?;
        let scheduler = system.actor_of(TimerScheduler, (), Props::default(), None)?;
        let actor = system.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let once = Schedule::Once {
            key: 0,
            delay: Duration::from_secs(1),
            message: DynamicMessage::user(OnOnceSchedule),
            receiver: actor.clone(),
        };
        scheduler.cast(once, ActorRef::no_sender());
        let fixed = Schedule::FixedRate {
            key: 1,
            initial_delay: None,
            delay: Duration::from_millis(500),
            factory: Box::new(FixedScheduleFactory),
            receiver: actor.clone(),
        };
        scheduler.cast(fixed, ActorRef::no_sender());
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }
}