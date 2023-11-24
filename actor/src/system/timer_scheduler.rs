use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use futures::task::ArcWake;
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;
use tracing::{debug, instrument, warn};

use actor_derive::EmptyCodec;

use crate::{Actor, DynamicMessage, Message};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::context::{ActorContext, Context};
use crate::props::Props;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct TimerSchedulerActor;

pub(crate) struct State {
    queue: DelayQueue<Schedule>,
    index: HashMap<u64, Key>,
    waker: futures::task::Waker,
}

#[async_trait]
impl Actor for TimerSchedulerActor {
    type S = State;
    type A = ();

    async fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        let myself = context.myself();
        debug!("{} pre start", myself);
        let waker = futures::task::waker(Arc::new(SchedulerWaker { scheduler: myself.clone() }));
        let state = State {
            queue: DelayQueue::new(),
            index: HashMap::new(),
            waker,
        };
        Ok(state)
    }
}

#[derive(Debug, Clone)]
struct ScheduleKey {
    index: u64,
    scheduler: ActorRef,
}

impl ScheduleKey {
    fn cancel(self) {
        self.scheduler.cast(CancelSchedule { index: self.index }, ActorRef::no_sender());
    }
}

#[derive(EmptyCodec)]
enum Schedule {
    Once {
        index: u64,
        delay: Duration,
        message: DynamicMessage,
        receiver: ActorRef,
    },
    OnceWith {
        index: u64,
        delay: Duration,
        block: Box<dyn FnOnce() + Send + 'static>,
    },
    FixedDelay {
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
        factory: Box<dyn Fn() -> DynamicMessage + Send + 'static>,
        receiver: ActorRef,
    },
    FixedDelayWith {
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
        factory: Box<dyn (Fn() -> Box<dyn Fn() + Send + 'static>) + Send + 'static>,
    },
}

impl Message for Schedule {
    type T = TimerSchedulerActor;

    #[instrument(skip_all)]
    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        match &*self {
            Schedule::Once { index, delay, .. } => {
                let index = *index;
                let delay = *delay;
                self.once(state, index, delay);
            }
            Schedule::FixedDelay { index, initial_delay, interval, .. } => {
                let index = *index;
                let initial_delay = *initial_delay;
                let interval = *interval;
                self.fixed_delay(state, index, initial_delay, interval);
            }
            Schedule::OnceWith { index, delay, .. } => {
                let index = *index;
                let delay = delay.clone();
                self.once(state, index, delay);
            }
            Schedule::FixedDelayWith { index, initial_delay, interval, .. } => {
                let index = *index;
                let initial_delay = *initial_delay;
                let interval = *interval;
                self.fixed_delay(state, index, initial_delay, interval);
            }
        }
        context.myself().cast(PollExpired, ActorRef::no_sender());
        Ok(())
    }
}

impl Schedule {
    fn once(self: Box<Self>, state: &mut State, index: u64, delay: Duration) {
        let delay_key = state.queue.insert(*self, delay);
        debug_assert!(!state.index.contains_key(&index));
        state.index.insert(index, delay_key);
    }

    fn fixed_delay(self: Box<Self>, state: &mut State, index: u64, initial_delay: Option<Duration>, interval: Duration) {
        let delay_key = match initial_delay {
            None => {
                let delay = interval;
                state.queue.insert(*self, delay)
            }
            Some(initial_delay) => {
                let delay = initial_delay;
                state.queue.insert(*self, delay)
            }
        };
        debug_assert!(!state.index.contains_key(&index));
        state.index.insert(index, delay_key);
    }
}

impl Debug for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Schedule::Once { index, delay, message, receiver } => {
                f.debug_struct("Once")
                    .field("index", index)
                    .field("delay", delay)
                    .field("message", message)
                    .field("receiver", receiver)
                    .finish()
            }
            Schedule::FixedDelay { index, initial_delay, interval, receiver, .. } => {
                f.debug_struct("FixedDelay")
                    .field("index", index)
                    .field("initial_delay", initial_delay)
                    .field("interval", interval)
                    .field("factory", &"..")
                    .field("receiver", receiver)
                    .finish()
            }
            Schedule::OnceWith { index, delay, .. } => {
                f.debug_struct("OnceWith")
                    .field("index", index)
                    .field("delay", delay)
                    .field("block", &"..")
                    .finish()
            }
            Schedule::FixedDelayWith { index, initial_delay, interval, .. } => {
                f.debug_struct("FixedDelayWith")
                    .field("index", index)
                    .field("initial_delay", initial_delay)
                    .field("interval", interval)
                    .field("factory", &"..")
                    .finish()
            }
        }
    }
}

#[derive(Debug, EmptyCodec)]
struct CancelSchedule {
    index: u64,
}

impl Message for CancelSchedule {
    type T = TimerSchedulerActor;

    #[instrument(skip_all)]
    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        match state.index.remove(&self.index) {
            None => {
                warn!("{} not found in TimerScheduler", self.index);
            }
            Some(key) => {
                state.queue.try_remove(&key);
                debug!("schedule index {} removed from TimerScheduler", self.index);
            }
        }
        context.myself().cast(PollExpired, ActorRef::no_sender());
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct CancelAllSchedule;

impl Message for CancelAllSchedule {
    type T = TimerSchedulerActor;

    #[instrument(skip_all)]
    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        state.index.clear();
        state.queue.clear();
        debug!("{} {:?}", context.myself(), self);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct PollExpired;

impl Message for PollExpired {
    type T = TimerSchedulerActor;

    #[instrument(skip_all)]
    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let waker = &state.waker;
        let queue = &mut state.queue;
        let index_map = &mut state.index;
        let mut ctx = futures::task::Context::from_waker(waker);

        while let Poll::Ready(Some(expired)) = queue.poll_expired(&mut ctx) {
            let schedule = expired.into_inner();
            match schedule {
                Schedule::Once { index, message, receiver, .. } => {
                    index_map.remove(&index);
                    receiver.tell(message, ActorRef::no_sender());
                }
                Schedule::FixedDelay { index, interval, factory, receiver, .. } => {
                    let message = factory();
                    receiver.tell(message, ActorRef::no_sender());
                    let next_delay = interval;
                    let reschedule = Schedule::FixedDelay {
                        index,
                        initial_delay: None,
                        interval,
                        factory,
                        receiver,
                    };
                    let new_key = queue.insert(reschedule, next_delay);
                    index_map.insert(index, new_key);
                }
                Schedule::OnceWith { index, block, .. } => {
                    index_map.remove(&index);
                    context.spawn(async move { block() });
                }
                Schedule::FixedDelayWith { index, interval, factory, .. } => {
                    let block = factory();
                    context.spawn(async move { block() });
                    let next_delay = interval;
                    let reschedule = Schedule::FixedDelayWith {
                        index,
                        initial_delay: None,
                        interval,
                        factory,
                    };
                    let new_key = queue.insert(reschedule, next_delay);
                    index_map.insert(index, new_key);
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

#[derive(Debug)]
pub struct TimerScheduler {
    index: AtomicU64,
    scheduler_actor: ActorRef,
}

impl TimerScheduler {
    pub fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let actor = context.actor_of(TimerSchedulerActor, (), Props::default(), Some("timer_scheduler".to_string()))?;
        Ok(Self {
            index: AtomicU64::new(0),
            scheduler_actor: actor,
        })
    }

    pub fn with_actor(actor: ActorRef) -> Self {
        Self {
            index: AtomicU64::new(0),
            scheduler_actor: actor,
        }
    }

    pub fn start_single_timer(&self, delay: Duration, message: DynamicMessage, receiver: ActorRef) -> ScheduleKey {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let once = Schedule::Once {
            index,
            delay,
            message,
            receiver,
        };
        self.scheduler_actor.cast(once, ActorRef::no_sender());
        ScheduleKey {
            index,
            scheduler: self.scheduler_actor.clone(),
        }
    }

    pub fn start_single_timer_with<F>(&self, delay: Duration, block: F) -> ScheduleKey
        where
            F: FnOnce() + Send + 'static {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let once_with = Schedule::OnceWith {
            index,
            delay,
            block: Box::new(block),
        };
        self.scheduler_actor.cast(once_with, ActorRef::no_sender());
        ScheduleKey {
            index,
            scheduler: self.scheduler_actor.clone(),
        }
    }

    pub fn start_timer_with_fixed_delay<F>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        factory: F,
        receiver: ActorRef,
    ) -> ScheduleKey
        where F: Fn() -> DynamicMessage + Send + 'static {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let fixed_delay = Schedule::FixedDelay {
            index,
            initial_delay,
            interval,
            factory: Box::new(factory),
            receiver,
        };
        self.scheduler_actor.cast(fixed_delay, ActorRef::no_sender());
        ScheduleKey {
            index,
            scheduler: self.scheduler_actor.clone(),
        }
    }

    pub fn start_timer_with_fixed_delay_with<F>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        factory_fn: F,
    ) -> ScheduleKey
        where
            F: Fn() -> Box<dyn Fn() + Send + 'static> + Send + 'static,
    {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let fixed_delay_with = Schedule::FixedDelayWith {
            index,
            initial_delay,
            interval,
            factory: Box::new(factory_fn),
        };
        self.scheduler_actor.cast(fixed_delay_with, ActorRef::no_sender());
        ScheduleKey {
            index,
            scheduler: self.scheduler_actor.clone(),
        }
    }

    pub fn cancel_all(&self) {
        self.scheduler_actor.cast(CancelAllSchedule, ActorRef::no_sender());
    }
}

#[cfg(test)]
mod scheduler_test {
    use std::time::Duration;

    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, DynamicMessage, EmptyTestActor, Message};
    use crate::context::{ActorContext, Context};
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;
    use crate::system::config::Config;
    use crate::system::timer_scheduler::{Schedule, TimerScheduler, TimerSchedulerActor};

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

    #[tokio::test]
    async fn test_scheduler() -> anyhow::Result<()> {
        let system = ActorSystem::create(Config::default()).await?;
        let scheduler_actor = system.actor_of(TimerSchedulerActor, (), Props::default(), None)?;
        let scheduler = TimerScheduler::with_actor(scheduler_actor);
        let actor = system.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let once = Schedule::Once {
            index: 0,
            delay: Duration::from_secs(1),
            message: DynamicMessage::user(OnOnceSchedule),
            receiver: actor.clone(),
        };
        scheduler.start_single_timer(
            Duration::from_secs(1),
            DynamicMessage::user(OnOnceSchedule),
            actor.clone());
        let key = scheduler.start_timer_with_fixed_delay(
            None,
            Duration::from_millis(500),
            || DynamicMessage::user(OnFixedSchedule),
            actor.clone());
        tokio::time::sleep(Duration::from_secs(4)).await;
        key.cancel();
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }
}