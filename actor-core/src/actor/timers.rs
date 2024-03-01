use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use futures::task::ArcWake;
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;
use tracing::{debug, error, trace, warn};

use actor_derive::EmptyCodec;

use crate::{Actor, DynMessage, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref::ActorRefExt;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::actor::props::Props;
use crate::message::terminated::Terminated;

#[derive(Debug)]
pub(crate) struct TimersActor {
    queue: DelayQueue<Schedule>,
    index: HashMap<u64, Key>,
    waker: futures::task::Waker,
    watching_receivers: HashMap<ActorRef, HashSet<u64>>,
}

impl TimersActor {
    pub(crate) fn new(myself: ActorRef) -> Self {
        let waker = futures::task::waker(Arc::new(SchedulerWaker { scheduler: myself }));
        Self {
            queue: DelayQueue::new(),
            index: HashMap::new(),
            waker,
            watching_receivers: HashMap::new(),
        }
    }
}

#[async_trait]
impl Actor for TimersActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} started", context.myself);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleKey {
    index: u64,
    scheduler: ActorRef,
    cancelled: Arc<AtomicBool>,
}

impl ScheduleKey {
    pub(crate) fn new(index: u64, scheduler: ActorRef) -> Self {
        Self {
            index,
            scheduler,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(self) {
        if !self.cancelled.swap(true, Ordering::Relaxed) {
            self.scheduler.cast_ns(CancelSchedule { index: self.index });
        }
    }
}

#[derive(EmptyCodec)]
enum Schedule {
    Once {
        index: u64,
        delay: Duration,
        message: DynMessage,
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
        message: DynMessage,
        receiver: ActorRef,
    },
    FixedDelayWith {
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
        block: Arc<Box<dyn Fn() + Send + Sync + 'static>>,
    },
}

#[async_trait]
impl Message for Schedule {
    type A = TimersActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match &*self {
            Schedule::Once { index, delay, receiver, .. } => {
                if !actor.watching_receivers.contains_key(receiver) {
                    context.watch(ReceiverTerminated(receiver.clone()));
                }
                let index = *index;
                let delay = *delay;
                self.once(actor, index, delay);
            }
            Schedule::FixedDelay { index, initial_delay, interval, receiver, .. } => {
                if !actor.watching_receivers.contains_key(receiver) {
                    context.watch(ReceiverTerminated(receiver.clone()));
                }
                let index = *index;
                let initial_delay = *initial_delay;
                let interval = *interval;
                self.fixed_delay(actor, index, initial_delay, interval);
            }
            Schedule::OnceWith { index, delay, .. } => {
                let index = *index;
                let delay = delay.clone();
                self.once(actor, index, delay);
            }
            Schedule::FixedDelayWith { index, initial_delay, interval, .. } => {
                let index = *index;
                let initial_delay = *initial_delay;
                let interval = *interval;
                self.fixed_delay(actor, index, initial_delay, interval);
            }
        }
        context.myself().cast(PollExpired, ActorRef::no_sender());
        Ok(())
    }
}

impl Schedule {
    fn once(self: Box<Self>, actor: &mut TimersActor, index: u64, delay: Duration) {
        let delay_key = actor.queue.insert(*self, delay);
        debug_assert!(!actor.index.contains_key(&index));
        actor.index.insert(index, delay_key);
    }

    fn fixed_delay(self: Box<Self>, actor: &mut TimersActor, index: u64, initial_delay: Option<Duration>, interval: Duration) {
        let delay_key = match initial_delay {
            None => {
                let delay = interval;
                actor.queue.insert(*self, delay)
            }
            Some(initial_delay) => {
                let delay = initial_delay;
                actor.queue.insert(*self, delay)
            }
        };
        debug_assert!(!actor.index.contains_key(&index));
        actor.index.insert(index, delay_key);
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
            Schedule::FixedDelay { index, initial_delay, interval, message, receiver } => {
                f.debug_struct("FixedDelay")
                    .field("index", index)
                    .field("initial_delay", initial_delay)
                    .field("interval", interval)
                    .field("message", message)
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

#[async_trait]
impl Message for CancelSchedule {
    type A = TimersActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match actor.index.remove(&self.index) {
            None => {
                warn!("{} not found in TimerScheduler", self.index);
            }
            Some(key) => {
                actor.queue.try_remove(&key);
                debug!("schedule index {} removed from TimerScheduler", self.index);
            }
        }
        actor.watching_receivers.retain(|receiver, index| {
            index.remove(&self.index);
            if index.is_empty() {
                context.unwatch(receiver);
                false
            } else {
                true
            }
        });
        context.myself().cast_ns(PollExpired);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct CancelAllSchedule;

#[async_trait]
impl Message for CancelAllSchedule {
    type A = TimersActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.index.clear();
        actor.queue.clear();
        for receiver in actor.watching_receivers.keys() {
            context.unwatch(receiver);
        }
        actor.watching_receivers.clear();
        debug!("{} {:?}", context.myself(), self);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct PollExpired;

#[async_trait]
impl Message for PollExpired {
    type A = TimersActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let waker = &actor.waker;
        let queue = &mut actor.queue;
        let index_map = &mut actor.index;
        let mut ctx = futures::task::Context::from_waker(waker);

        while let Poll::Ready(Some(expired)) = queue.poll_expired(&mut ctx) {
            let schedule = expired.into_inner();
            match schedule {
                Schedule::Once { index, message, receiver, .. } => {
                    index_map.remove(&index);
                    receiver.tell(message, ActorRef::no_sender());
                }
                Schedule::FixedDelay { index, interval, message, receiver, .. } => {
                    match message.dyn_clone() {
                        None => {
                            error!("fixed delay with message {:?} not impl dyn_clone, message cannot be cloned", message);
                        }
                        Some(message) => {
                            receiver.tell(message, ActorRef::no_sender());
                        }
                    }
                    let next_delay = interval;
                    let reschedule = Schedule::FixedDelay {
                        index,
                        initial_delay: None,
                        interval,
                        message,
                        receiver,
                    };
                    let new_key = queue.insert(reschedule, next_delay);
                    index_map.insert(index, new_key);
                }
                Schedule::OnceWith { index, block, .. } => {
                    index_map.remove(&index);
                    context.spawn_fut(async move { block() });
                }
                Schedule::FixedDelayWith { index, interval, block, .. } => {
                    let block_clone = block.clone();
                    context.spawn_fut(async move { block_clone() });
                    let next_delay = interval;
                    let reschedule = Schedule::FixedDelayWith {
                        index,
                        initial_delay: None,
                        interval,
                        block,
                    };
                    let new_key = queue.insert(reschedule, next_delay);
                    index_map.insert(index, new_key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct ReceiverTerminated(ActorRef);

#[async_trait]
impl Message for ReceiverTerminated {
    type A = TimersActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(index_set) = actor.watching_receivers.remove(&self.0) {
            for index in &index_set {
                if let Some(key) = actor.index.remove(&index) {
                    actor.queue.try_remove(&key);
                }
            }
            let index_str = index_set.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            trace!("{} watch receiver {} stopped, stop associated timers {:?}", context.myself(), self.0, index_str);
        }
        Ok(())
    }
}

impl Terminated for ReceiverTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
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

#[derive(Debug, Clone)]
pub struct Timers {
    index: Arc<AtomicU64>,
    scheduler_actor: ActorRef,
}

impl Timers {
    pub fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let scheduler_actor = context
            .spawn(
                Props::new_with_ctx(move |context| Ok(TimersActor::new(context.myself.clone()))),
                "timers",
            )?;
        Ok(Self {
            index: AtomicU64::new(0).into(),
            scheduler_actor,
        })
    }

    pub fn with_actor(timers: ActorRef) -> Self {
        Self {
            index: AtomicU64::new(0).into(),
            scheduler_actor: timers,
        }
    }

    pub fn start_single_timer(&self, delay: Duration, message: DynMessage, receiver: ActorRef) -> ScheduleKey {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let once = Schedule::Once {
            index,
            delay,
            message,
            receiver,
        };
        self.scheduler_actor.cast_ns(once);
        ScheduleKey::new(index, self.scheduler_actor.clone())
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
        self.scheduler_actor.cast_ns(once_with);
        ScheduleKey::new(index, self.scheduler_actor.clone())
    }

    pub fn start_timer_with_fixed_delay(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        message: DynMessage,
        receiver: ActorRef,
    ) -> ScheduleKey {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let fixed_delay = Schedule::FixedDelay {
            index,
            initial_delay,
            interval,
            message,
            receiver,
        };
        self.scheduler_actor.cast_ns(fixed_delay);
        ScheduleKey::new(index, self.scheduler_actor.clone())
    }

    pub fn start_timer_with_fixed_delay_with<F>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        block: F,
    ) -> ScheduleKey
        where
            F: Fn() + Send + Sync + 'static,
    {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let fixed_delay_with = Schedule::FixedDelayWith {
            index,
            initial_delay,
            interval,
            block: Arc::new(Box::new(block)),
        };
        self.scheduler_actor.cast_ns(fixed_delay_with);
        ScheduleKey::new(index, self.scheduler_actor.clone())
    }

    pub fn cancel_all(&self) {
        self.scheduler_actor.cast_ns(CancelAllSchedule);
    }
}

// #[cfg(test)]
// mod scheduler_test {
//     use std::time::Duration;
//
//     use tracing::info;
//
//     use actor_derive::{CloneableEmptyCodec, EmptyCodec};
//
//     use crate::{DynMessage, EmptyTestActor, Message};
//     use crate::actor::context::{ActorContext, Context};
//     use crate::props::Props;
//     use crate::system::ActorSystem;
//     use crate::system::config::ActorSystemConfig;
//     use crate::system::timer_scheduler::{TimerScheduler, TimerSchedulerActor};
//
//     #[derive(Debug, EmptyCodec)]
//     struct OnOnceSchedule;
//
//     impl Message for OnOnceSchedule {
//         type A = EmptyTestActor;
//
//         fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//             info!("{} OnOnceSchedule", context.myself());
//             Ok(())
//         }
//     }
//
//     #[derive(Debug, Clone, CloneableEmptyCodec)]
//     struct OnFixedSchedule;
//
//     impl Message for OnFixedSchedule {
//         type A = EmptyTestActor;
//
//         fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//             info!("{} OnFixedSchedule", context.myself());
//             Ok(())
//         }
//     }
//
//     #[tokio::test]
//     async fn test_scheduler() -> anyhow::Result<()> {
//         let system = ActorSystem::create(ActorSystemConfig::default()).await?;
//         let scheduler_actor = system.spawn_anonymous_actor(Props::create(|context| TimerSchedulerActor::new(context.myself.clone())))?;
//         let scheduler = TimerScheduler::with_actor(scheduler_actor);
//         let actor = system.spawn_anonymous_actor(Props::create(|_| EmptyTestActor))?;
//         scheduler.start_single_timer(
//             Duration::from_secs(1),
//             DynMessage::user(OnOnceSchedule),
//             actor.clone());
//         let key = scheduler.start_timer_with_fixed_delay(
//             None,
//             Duration::from_millis(500),
//             DynMessage::user(OnFixedSchedule),
//             actor.clone());
//         tokio::time::sleep(Duration::from_secs(4)).await;
//         key.cancel();
//         tokio::time::sleep(Duration::from_secs(10)).await;
//         Ok(())
//     }
// }