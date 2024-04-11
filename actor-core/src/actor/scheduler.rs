use std::collections::HashMap;
use std::ops::{Deref, Sub};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures::StreamExt;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::Instant;
use tokio_util::time::delay_queue::{Expired, Key};
use tokio_util::time::DelayQueue;
use tracing::{debug, trace, warn};

enum Schedule {
    Once(Once),
    FixedDelay(FixedDelay),
    FixedRate(FixedRate),
    Cancel(u64),
    CancelALl,
}

struct Once {
    index: u64,
    delay: Duration,
    block: Box<dyn FnOnce() + Send + 'static>,
}

struct FixedDelay {
    index: u64,
    initial_delay: Option<Duration>,
    interval: Duration,
    block: Box<dyn Fn() + Send + 'static>,
}

struct FixedRate {
    index: u64,
    initial_delay: Option<Duration>,
    interval: Duration,
    actual_delay: Duration,
    block: Box<dyn Fn() + Send + 'static>,
}

impl Schedule {
    fn once(index: u64, delay: Duration, block: Box<dyn FnOnce() + Send>) -> Self {
        Schedule::Once(Once { index, delay, block })
    }

    fn fixed_delay(
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
        block: Box<dyn Fn() + Send>,
    ) -> Self {
        Schedule::FixedDelay(FixedDelay { index, initial_delay, interval, block })
    }

    fn fixed_rate(
        index: u64,
        initial_delay: Option<Duration>,
        interval: Duration,
        actual_delay: Duration,
        block: Box<dyn Fn() + Send>,
    ) -> Self {
        Schedule::FixedRate(FixedRate { index, initial_delay, interval, actual_delay, block })
    }

    fn cancel(index: u64) -> Self {
        Schedule::Cancel(index)
    }

    fn cancel_all() -> Self {
        Schedule::CancelALl
    }
}

struct Scheduler {
    rx: UnboundedReceiver<Schedule>,
    queue: DelayQueue<Schedule>,
    index: HashMap<u64, Key>,
}

impl Scheduler {
    fn run(self) {
        tokio::spawn(async move {
            let Scheduler { mut rx, mut queue, mut index } = self;
            loop {
                select! {
                    Some(schedule) = rx.recv() => {
                        Self::handle_schedule(schedule, &mut queue, &mut index);
                    }
                    Some(expired) = queue.next() => {
                        Self::handle_expired(expired, &mut queue, &mut index);
                    }
                    else => {
                        break;
                    }
                }
            }
        });
    }

    fn handle_schedule(schedule: Schedule, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        match schedule {
            Schedule::Once(once) => {
                Self::on_once_schedule(once, queue, index_map);
            }
            Schedule::FixedDelay(fixed_delay) => {
                Self::on_fixed_delay_schedule(fixed_delay, queue, index_map);
            }
            Schedule::FixedRate(fixed_rate) => {
                Self::on_fixed_rate_schedule(fixed_rate, queue, index_map);
            }
            Schedule::Cancel(index) => {
                Self::on_cancel(index, queue, index_map);
            }
            Schedule::CancelALl => {
                Self::on_cancel_all(queue, index_map);
            }
        }
    }

    fn on_once_schedule(once: Once, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        let index = once.index;
        let timeout = once.delay;
        let key = queue.insert(Schedule::Once(once), timeout);
        trace!("schedule once with index {}  after {:?}", index,  timeout);
        index_map.insert(index, key);
    }

    fn on_fixed_delay_schedule(fixed_delay: FixedDelay, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        let index = fixed_delay.index;
        let timeout = fixed_delay.initial_delay.unwrap_or(fixed_delay.interval);
        let key = queue.insert(Schedule::FixedDelay(fixed_delay), timeout);
        trace!("schedule fixed delay with index {} after {:?}", index,  timeout);
        index_map.insert(index, key);
    }

    fn on_fixed_rate_schedule(fixed_rate: FixedRate, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        let index = fixed_rate.index;
        let timeout = fixed_rate.initial_delay.unwrap_or(fixed_rate.interval);
        let key = queue.insert(Schedule::FixedRate(fixed_rate), timeout);
        trace!("schedule fixed rate with index {} after {:?}", index,  timeout);
        index_map.insert(index, key);
    }

    fn on_cancel(index: u64, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        match index_map.remove(&index) {
            None => {
                warn!("cancel a not exists schedule {}", index);
            }
            Some(key) => {
                match queue.try_remove(&key) {
                    None => {
                        warn!("{} already executed, cancel failed", index);
                    }
                    Some(_) => {
                        debug!("{} cancel success", index);
                    }
                }
            }
        }
    }

    fn on_cancel_all(queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        queue.clear();
        index_map.clear();
    }

    fn handle_expired(expired: Expired<Schedule>, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        let deadline = expired.deadline();
        let schedule = expired.into_inner();
        match schedule {
            Schedule::Once(once) => {
                Self::handle_once_expired(once, index_map);
            }
            Schedule::FixedDelay(fixed_delay) => {
                Self::handle_fixed_delay_expired(fixed_delay, queue, index_map);
            }
            Schedule::FixedRate(fixed_rate) => {
                Self::handle_fixed_rate_expired(fixed_rate, deadline, queue, index_map);
            }
            _ => {}
        }
    }

    fn handle_once_expired(once: Once, index_map: &mut HashMap<u64, Key>) {
        let Once { index, delay, block } = once;
        trace!("execute once expired task {} after {:?}", index, delay);
        index_map.remove(&index);
        block();
    }

    fn handle_fixed_delay_expired(fixed_delay: FixedDelay, queue: &mut DelayQueue<Schedule>, index_map: &mut HashMap<u64, Key>) {
        let FixedDelay { index, initial_delay, interval, block } = fixed_delay;
        let current_delay = initial_delay.unwrap_or(interval);
        trace!("execute fixed delay expired task {} after {:?}", index, current_delay);
        block();
        let next_schedule = Schedule::fixed_delay(index, None, interval, block);
        let next_key = queue.insert(next_schedule, interval);
        index_map.insert(index, next_key);
    }

    fn handle_fixed_rate_expired(
        fixed_rate: FixedRate,
        deadline: Instant,
        queue: &mut DelayQueue<Schedule>,
        index_map: &mut HashMap<u64, Key>,
    ) {
        let FixedRate {
            index,
            initial_delay,
            interval,
            actual_delay,
            block
        } = fixed_rate;
        let current_delay = initial_delay.unwrap_or(interval);
        trace!("execute fixed rate expired task {} after {:?} with actual delay {:?}", index, current_delay, actual_delay);
        block();
        let next_actual_delay = interval.sub(deadline.elapsed());
        let next_schedule = Schedule::fixed_rate(index, None, interval, next_actual_delay, block);
        let next_key = queue.insert(next_schedule, interval);
        index_map.insert(index, next_key);
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleKey {
    index: u64,
    sender: UnboundedSender<Schedule>,
}

impl ScheduleKey {
    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn cancel(self) {
        let ScheduleKey { index, sender } = self;
        if sender.send(Schedule::cancel(index)).is_err() {
            warn!("cancel {} failed, scheduler closed", index);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerSender {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    index: AtomicU64,
    sender: UnboundedSender<Schedule>,
}

impl Deref for SchedulerSender {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl SchedulerSender {
    fn next_index(&self) -> u64 {
        self.index.fetch_add(1, Ordering::Relaxed)
    }

    fn key(&self, index: u64) -> ScheduleKey {
        ScheduleKey {
            index,
            sender: self.sender.clone(),
        }
    }

    pub fn schedule_once<F>(&self, delay: Duration, block: F) -> ScheduleKey where F: FnOnce() + Send + 'static {
        let index = self.next_index();
        let schedule = Schedule::once(index, delay, Box::new(block));
        if self.sender.send(schedule).is_err() {
            warn!("schedule once {} failed, scheduler closed", index);
        }
        self.key(index)
    }

    pub fn schedule_with_fixed_delay<F>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        block: F,
    ) -> ScheduleKey
        where F: Fn() + Send + 'static,
    {
        let index = self.next_index();
        let schedule = Schedule::fixed_delay(index, initial_delay, interval, Box::new(block));
        if self.sender.send(schedule).is_err() {
            warn!("schedule once {} failed, scheduler closed", index);
        }
        self.key(index)
    }

    pub fn schedule_with_fixed_rate<F>(
        &self,
        initial_delay: Option<Duration>,
        interval: Duration,
        block: F,
    ) -> ScheduleKey
        where F: Fn() + Send + 'static
    {
        let index = self.next_index();
        let schedule = Schedule::fixed_rate(
            index,
            initial_delay,
            interval,
            initial_delay.unwrap_or(interval),
            Box::new(block),
        );
        if self.sender.send(schedule).is_err() {
            warn!("schedule once {} failed, scheduler closed", index);
        }
        self.key(index)
    }

    pub fn cancel_all(&self) {
        if self.sender.send(Schedule::cancel_all()).is_err() {
            warn!("cancel all failed, scheduler closed");
        }
    }
}

pub fn scheduler() -> SchedulerSender {
    let (tx, rx) = unbounded_channel();
    let scheduler = Scheduler {
        rx,
        queue: DelayQueue::new(),
        index: HashMap::new(),
    };
    scheduler.run();
    let sender = SchedulerSender {
        inner: Arc::new(
            Inner {
                index: AtomicU64::new(0),
                sender: tx,
            }
        )
    };
    sender
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tracing::Level;

    use crate::actor::scheduler::scheduler;
    use crate::ext::init_logger;

    #[tokio::test]
    async fn test_scheduler() -> eyre::Result<()> {
        init_logger(Level::DEBUG);
        let scheduler = scheduler();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        scheduler.schedule_once(Duration::from_secs(1), move || {
            let _ = tx.send(());
        });
        tokio::time::timeout(Duration::from_millis(1050), rx).await??;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let key = scheduler.schedule_once(Duration::from_secs(1), move || {
            let _ = tx.send(());
        });
        key.cancel();
        assert!(rx.await.is_err());
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let key = scheduler.schedule_with_fixed_delay(
            Some(Duration::from_secs(1)),
            Duration::from_secs(1),
            move || {
                let _ = tx.try_send(());
            });
        for _ in 0..3 {
            assert!(tokio::time::timeout(Duration::from_secs(2050), rx.recv()).await?.is_some());
        }
        key.cancel();
        assert!(rx.recv().await.is_none());
        Ok(())
    }
}