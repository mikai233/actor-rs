use std::any::type_name;
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use actor_derive::Message;
use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use futures::task::ArcWake;
use itertools::Itertools;
use tokio_util::time::delay_queue::Key;
use tokio_util::time::DelayQueue;
use tracing::{debug, error, trace};

use crate::actor::context::{ActorContext, Context};
use crate::actor::props::Props;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::message::handler::MessageHandler;
use crate::message::terminated::Terminated;
use crate::message::{DynMessage, Message};

use super::behavior::Behavior;
use super::receive::Receive;
use super::Actor;

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

    fn watch_receiver(
        watching_receivers: &mut HashMap<ActorRef, HashSet<u64>>,
        context: &mut Context,
        receiver: ActorRef,
        index: u64,
    ) -> anyhow::Result<()> {
        match watching_receivers.entry(receiver) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(index);
            }
            Entry::Vacant(v) => {
                let mut indices = HashSet::new();
                indices.insert(index);
                v.insert(indices);
            }
        }
        Ok(())
    }

    fn unwatch_receiver(
        watching_receivers: &mut HashMap<ActorRef, HashSet<u64>>,
        context: &mut Context,
        index: u64,
    ) {
        watching_receivers.retain(|receiver, indexes| {
            indexes.remove(&index);
            if indexes.is_empty() {
                context.unwatch(receiver);
                false
            } else {
                true
            }
        });
    }

    fn once(&mut self, schedule: Schedule, index: u64, delay: Duration) {
        let key = self.queue.insert(schedule, delay);
        debug_assert!(!self.index.contains_key(&index));
        self.index.insert(index, key);
    }

    fn fixed_delay(
        &mut self,
        schedule: Schedule,
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
    ) {
        let key = match initial_delay {
            None => {
                let delay = interval;
                self.queue.insert(schedule, delay)
            }
            Some(initial_delay) => {
                let delay = initial_delay;
                self.queue.insert(schedule, delay)
            }
        };
        debug_assert!(!self.index.contains_key(&index));
        self.index.insert(index, key);
    }

    fn handle_terminated(
        actor: &mut TimersActor,
        ctx: &mut Context,
        message: Terminated,
        sender: Option<ActorRef>,
        _: &Receive<Self>,
    ) -> anyhow::Result<Behavior<Self>> {
        let watchee = message.actor;
        if let Some(indices) = actor.watching_receivers.remove(&watchee) {
            for index in &indices {
                if let Some(key) = actor.index.remove(&index) {
                    actor.queue.try_remove(&key);
                }
            }
            let indices = indices.iter().join(", ");
            trace!(
                "{} watch receiver {} stopped, stop associated timers {}",
                ctx.myself(),
                watchee,
                indices
            );
        }
        Ok(Behavior::same())
    }
}

impl Actor for TimersActor {
    type Context = Context;

    fn receive(&self) -> Receive<Self> {
        Receive::new()
            .handle::<Schedule>()
            .handle::<CancelSchedule>()
            .handle::<CancelAllSchedule>()
            .handle::<PollExpired>()
            .is::<Terminated>(Self::handle_terminated)
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleKey {
    index: u64,
    signature: &'static str,
    scheduler: ActorRef,
    cancelled: Arc<AtomicBool>,
}

impl ScheduleKey {
    pub(crate) fn new(signature: &'static str, index: u64, scheduler: ActorRef) -> Self {
        Self {
            index,
            signature,
            scheduler,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(self) {
        if !self.cancelled.swap(true, Ordering::Relaxed) {
            self.scheduler.cast_ns(CancelSchedule {
                index: self.index,
                signature: self.signature,
            });
        }
    }
}

impl PartialEq for ScheduleKey {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.signature == other.signature
    }
}

impl Eq for ScheduleKey {}

impl PartialOrd for ScheduleKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl Ord for ScheduleKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

#[derive(Message)]
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
        block: Box<dyn Fn() + Send + Sync + 'static>,
    },
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Schedule::Once {
                index,
                delay,
                message,
                receiver,
            } => {
                write!(
                    f,
                    "Schedule::Once {{ index: {}, delay: {:?}, message: {}, receiver: {} }}",
                    index, delay, message, receiver
                )
            }
            Schedule::OnceWith {
                index,
                delay,
                block,
            } => {
                write!(
                    f,
                    "Schedule::OnceWith {{ index: {}, delay: {:?}, block: ..",
                    index, delay
                )
            }
            Schedule::FixedDelay {
                index,
                initial_delay,
                interval,
                message,
                receiver,
            } => {
                write!(
                    f,
                    "Schedule::FixedDelay {{ index: {}, initial_delay: {:?}, interval: {:?}, message: {}, receiver: {}",
                    index,
                    initial_delay,
                    interval,
                    message,
                    receiver,
                )
            }
            Schedule::FixedDelayWith {
                index,
                initial_delay,
                interval,
                block,
            } => {
                write!(
                    f,
                    "Schedule::FixedDelayWith {{ index: {}, initial_delay: {:?}, interval: {:?}, block: ..",
                    index,
                    initial_delay,
                    interval,
                )
            }
        }
    }
}

impl MessageHandler<TimersActor> for Schedule {
    fn handle(
        actor: &mut TimersActor,
        ctx: &mut <TimersActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<TimersActor>,
    ) -> anyhow::Result<Behavior<TimersActor>> {
        match &message {
            Schedule::Once {
                index,
                delay,
                receiver,
                ..
            } => {
                TimersActor::watch_receiver(&mut actor.watching_receivers, ctx, receiver, index)?;
                actor.once(message, index, delay);
            }
            Schedule::FixedDelay {
                index,
                initial_delay,
                interval,
                receiver,
                ..
            } => {
                TimersActor::watch_receiver(&mut actor.watching_receivers, ctx, receiver, *index)?;
                actor.fixed_delay(message, index, initial_delay, interval);
            }
            Schedule::OnceWith { index, delay, .. } => {
                actor.once(message, index, delay);
            }
            Schedule::FixedDelayWith {
                index,
                initial_delay,
                interval,
                ..
            } => {
                actor.fixed_delay(message, index, initial_delay, interval);
            }
        }
        ctx.myself.cast_ns(PollExpired);
        Ok(Behavior::same())
    }
}

impl Debug for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Schedule::Once {
                index,
                delay,
                message,
                receiver,
            } => f
                .debug_struct("Once")
                .field("index", index)
                .field("delay", delay)
                .field("message", message)
                .field("receiver", receiver)
                .finish(),
            Schedule::FixedDelay {
                index,
                initial_delay,
                interval,
                message,
                receiver,
            } => f
                .debug_struct("FixedDelay")
                .field("index", index)
                .field("initial_delay", initial_delay)
                .field("interval", interval)
                .field("message", message)
                .field("receiver", receiver)
                .finish(),
            Schedule::OnceWith { index, delay, .. } => f
                .debug_struct("OnceWith")
                .field("index", index)
                .field("delay", delay)
                .finish_non_exhaustive(),
            Schedule::FixedDelayWith {
                index,
                initial_delay,
                interval,
                ..
            } => f
                .debug_struct("FixedDelayWith")
                .field("index", index)
                .field("initial_delay", initial_delay)
                .field("interval", interval)
                .finish_non_exhaustive(),
        }
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("CancelSchedule {{ index: {index}, signature: {signature}")]
struct CancelSchedule {
    index: u64,
    signature: &'static str,
}

impl MessageHandler<TimersActor> for CancelSchedule {
    fn handle(
        actor: &mut TimersActor,
        ctx: &mut <TimersActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<TimersActor>,
    ) -> anyhow::Result<Behavior<TimersActor>> {
        let Self {
            index,
            signature: message,
        } = message;
        match actor.index.remove(&index) {
            None => {
                debug!("{}[{}] not found in TimerScheduler", message, index);
            }
            Some(key) => {
                actor.queue.try_remove(&key);
                debug!("schedule index {} removed from TimerScheduler", index);
            }
        }
        TimersActor::unwatch_receiver(&mut actor.watching_receivers, ctx.context_mut(), index);
        ctx.context().myself.cast_ns(PollExpired);
        Ok(Behavior::same())
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("CancelAllSchedule")]
struct CancelAllSchedule;

impl MessageHandler<TimersActor> for CancelAllSchedule {
    fn handle(
        actor: &mut TimersActor,
        ctx: &mut <TimersActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<TimersActor>,
    ) -> anyhow::Result<Behavior<TimersActor>> {
        actor.index.clear();
        actor.queue.clear();
        for receiver in actor.watching_receivers.keys() {
            ctx.context_mut().unwatch(receiver);
        }
        actor.watching_receivers.clear();
        let myself = ctx.context().myself();
        debug!("{} {}", myself, message);
        Ok(Behavior::same())
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("PollExpired")]
struct PollExpired;

impl MessageHandler<TimersActor> for PollExpired {
    fn handle(
        actor: &mut TimersActor,
        ctx: &mut <TimersActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<TimersActor>,
    ) -> anyhow::Result<Behavior<TimersActor>> {
        let waker = &actor.waker;
        let queue = &mut actor.queue;
        let indexes = &mut actor.index;
        let mut task_ctx = futures::task::Context::from_waker(waker);

        while let Poll::Ready(Some(expired)) = queue.poll_expired(&mut task_ctx) {
            let schedule = expired.into_inner();
            match schedule {
                Schedule::Once {
                    index,
                    message,
                    receiver,
                    ..
                } => {
                    indexes.remove(&index);
                    receiver.cast_ns(message);
                    TimersActor::unwatch_receiver(
                        &mut actor.watching_receivers,
                        ctx.context_mut(),
                        index,
                    );
                }
                Schedule::FixedDelay {
                    index,
                    interval,
                    message,
                    receiver,
                    ..
                } => {
                    match message.dyn_clone() {
                        Ok(message) => {
                            receiver.cast_ns(message);
                        }
                        Err(_) => {
                            error!("fixed delay with message {:?} not impl dyn_clone, message cannot be cloned", message);
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
                    indexes.insert(index, new_key);
                }
                Schedule::OnceWith { index, block, .. } => {
                    indexes.remove(&index);
                    block();
                }
                Schedule::FixedDelayWith {
                    index,
                    interval,
                    block,
                    ..
                } => {
                    block();
                    let next_delay = interval;
                    let reschedule = Schedule::FixedDelayWith {
                        index,
                        initial_delay: None,
                        interval,
                        block,
                    };
                    let new_key = queue.insert(reschedule, next_delay);
                    indexes.insert(index, new_key);
                }
            }
        }
        Ok(Behavior::same())
    }
}

#[derive(Debug)]
struct SchedulerWaker {
    scheduler: ActorRef,
}

impl ArcWake for SchedulerWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.scheduler.cast_ns(PollExpired);
    }
}

#[derive(Debug, Clone)]
pub struct Timers {
    index: Arc<AtomicU64>,
    scheduler_actor: ActorRef,
}

impl Timers {
    pub fn new(context: &mut Context) -> anyhow::Result<Self> {
        let scheduler_actor = context.spawn(
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

    pub fn start_single_timer<M>(
        &self,
        delay: Duration,
        message: M,
        receiver: ActorRef,
    ) -> ScheduleKey
    where
        M: Message,
    {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let once = Schedule::Once {
            index,
            delay,
            message: Box::new(message),
            receiver,
        };
        self.scheduler_actor.cast_ns(once);
        ScheduleKey::new(
            M::signature_sized().name,
            index,
            self.scheduler_actor.clone(),
        )
    }

    pub fn start_single_timer_with<F>(&self, delay: Duration, block: F) -> ScheduleKey
    where
        F: FnOnce() + Send + 'static,
    {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let once_with = Schedule::OnceWith {
            index,
            delay,
            block: Box::new(block),
        };
        self.scheduler_actor.cast_ns(once_with);
        ScheduleKey::new(type_name::<F>(), index, self.scheduler_actor.clone())
    }

    pub fn start_timer_with_fixed_delay<M>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        message: M,
        receiver: ActorRef,
    ) -> ScheduleKey
    where
        M: Message + Clone,
    {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        let fixed_delay = Schedule::FixedDelay {
            index,
            initial_delay,
            interval,
            message: Box::new(message),
            receiver,
        };
        self.scheduler_actor.cast_ns(fixed_delay);
        ScheduleKey::new(
            M::signature_sized().name,
            index,
            self.scheduler_actor.clone(),
        )
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
            block: Box::new(block),
        };
        self.scheduler_actor.cast_ns(fixed_delay_with);
        ScheduleKey::new(type_name::<F>(), index, self.scheduler_actor.clone())
    }

    pub fn cancel_all(&self) {
        self.scheduler_actor.cast_ns(CancelAllSchedule);
    }
}
