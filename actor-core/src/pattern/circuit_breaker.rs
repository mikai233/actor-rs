use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use rand::random;
use tracing::debug;

use crate::actor::scheduler::SchedulerSender;

pub struct CircuitBreaker {
    state: State,
    scheduler: SchedulerSender,
    max_failures: usize,
    call_timeout: Duration,
    reset_timeout: Duration,
    max_reset_timeout: Duration,
    exponential_backoff_factor: f64,
    random_factor: f64,
    current_reset_timeout: Duration,
    on_open_listeners: Vec<Box<dyn Fn() + Send>>,
    on_half_open_listeners: Vec<Box<dyn Fn() + Send>>,
    on_close_listeners: Vec<Box<dyn Fn() + Send>>,
    call_failure_listeners: Vec<Box<dyn Fn(SystemTime) + Send>>,
    call_timeout_listeners: Vec<Box<dyn Fn(SystemTime) + Send>>,
    call_breaker_open_listeners: Vec<Box<dyn Fn() + Send>>,
    call_success_listeners: Vec<Box<dyn Fn() + Send>>,
    change_to_half_open: Arc<Box<dyn Fn() + Send + Sync>>,
}

impl Debug for CircuitBreaker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("state", &self.state)
            .field("max_failures", &self.max_failures)
            .field("call_timeout", &self.call_timeout)
            .field("reset_timeout", &self.reset_timeout)
            .field("max_reset_timeout", &self.max_reset_timeout)
            .field("exponential_backoff_factor", &self.exponential_backoff_factor)
            .field("random_factor", &self.random_factor)
            .field("current_reset_timeout", &self.current_reset_timeout)
            .finish_non_exhaustive()
    }
}

impl CircuitBreaker {
    fn new<F>(
        scheduler: SchedulerSender,
        max_failures: usize,
        call_timeout: Duration,
        reset_timeout: Duration,
        max_reset_timeout: Duration,
        exponential_backoff_factor: f64,
        random_factor: f64,
        change_to_half_open: F,
    ) -> anyhow::Result<Self> where F: Fn() + Send + Sync + 'static {
        if exponential_backoff_factor < 1.0 {
            return Err(anyhow!("exponential_backoff_factor must be >= 1.0"));
        }
        if random_factor < 0.0 || random_factor > 1.0 {
            return Err(anyhow!("random_factor must be between 0.0 and 1.0"));
        }
        let myself = Self {
            state: State::Closed { failure_counter: 0 },
            scheduler,
            max_failures,
            call_timeout,
            reset_timeout,
            max_reset_timeout,
            exponential_backoff_factor,
            random_factor,
            current_reset_timeout: reset_timeout,
            on_open_listeners: vec![],
            on_half_open_listeners: vec![],
            on_close_listeners: vec![],
            call_failure_listeners: vec![],
            call_timeout_listeners: vec![],
            call_breaker_open_listeners: vec![],
            call_success_listeners: vec![],
            change_to_half_open: Arc::new(Box::new(change_to_half_open)),
        };
        Ok(myself)
    }

    fn transition_listeners(&self, state: State) -> &Vec<Box<dyn Fn() + Send>> {
        match state {
            State::Closed { .. } => {
                &self.on_close_listeners
            }
            State::HalfOpen => {
                &self.on_half_open_listeners
            }
            State::Open { .. } => {
                &self.on_open_listeners
            }
        }
    }

    fn transition_listeners_mut(&mut self, state: State) -> &mut Vec<Box<dyn Fn() + Send>> {
        match state {
            State::Closed { .. } => {
                &mut self.on_close_listeners
            }
            State::HalfOpen => {
                &mut self.on_half_open_listeners
            }
            State::Open { .. } => {
                &mut self.on_open_listeners
            }
        }
    }

    fn notify_transition_listeners(&self, state: State) {
        for listener in self.transition_listeners(state) {
            listener();
        }
    }

    fn notify_call_timeout_listeners(&self) {
        for listener in &self.call_timeout_listeners {
            listener(SystemTime::now());
        }
    }

    fn notify_call_success_listeners(&self) {
        for listener in &self.call_success_listeners {
            listener();
        }
    }

    fn notify_call_failure_listeners(&self) {
        for listener in &self.call_failure_listeners {
            listener(SystemTime::now());
        }
    }

    fn notify_call_breaker_open_listeners(&self) {
        for listener in &self.call_breaker_open_listeners {
            listener();
        }
    }

    async fn call_through<Fut, R>(&mut self, body: Fut) -> anyhow::Result<R> where Fut: Future<Output=anyhow::Result<R>> {
        match tokio::time::timeout(self.call_timeout, body).await {
            Ok(Ok(result)) => {
                self.notify_call_success_listeners();
                self.call_succeeds();
                Ok(result)
            }
            Ok(Err(error)) => {
                self.notify_call_failure_listeners();
                self.call_fails();
                Err(error)
            }
            Err(_) => {
                self.notify_call_timeout_listeners();
                self.call_fails();
                Err(anyhow!("call timeout after {:?}", self.call_timeout))
            }
        }
    }

    async fn invoke<Fut, R>(&mut self, body: Fut) -> anyhow::Result<R> where Fut: Future<Output=anyhow::Result<R>> {
        match self.state {
            State::Closed { .. } => {
                self.call_through(body).await
            }
            State::HalfOpen => {
                self.call_through(body).await
            }
            State::Open { open_time, current_reset_timeout } => {
                let duration_from_opened = SystemTime::now().duration_since(open_time)
                    .expect("open_time is later than now");
                if duration_from_opened >= current_reset_timeout {
                    Err(anyhow!("breaker is open, will change to half open immediately"))
                } else {
                    let remain = current_reset_timeout - duration_from_opened;
                    Err(anyhow!("breaker is open, wll change to half open after {:?}", remain))
                }
            }
        }
    }

    fn call_succeeds(&mut self) {
        let state = self.state;
        match state {
            State::Closed { .. } => {
                self.state = State::Closed { failure_counter: 0 }
            }
            State::HalfOpen => {
                self.transition(state, State::Closed { failure_counter: 0 });
            }
            State::Open { .. } => {}
        }
    }

    fn call_fails(&mut self) {
        debug!("{:?}", self);
        match &mut self.state {
            State::Closed { failure_counter } => {
                *failure_counter += 1;
                if *failure_counter >= self.max_failures {
                    self.transition(
                        self.state.clone(),
                        State::Open {
                            open_time: SystemTime::now(),
                            current_reset_timeout: self.reset_timeout,
                        },
                    );
                }
            }
            State::HalfOpen => {
                self.transition(
                    self.state.clone(),
                    State::Open {
                        open_time: SystemTime::now(),
                        current_reset_timeout: self.reset_timeout,
                    },
                );
            }
            State::Open { .. } => {}
        }
    }

    fn transition(&mut self, from_state: State, to_state: State) {
        if self.state == from_state {
            self.state = to_state;
            self.enter();
            self.notify_transition_listeners(to_state);
        }
    }

    fn enter(&mut self) {
        match &mut self.state {
            State::Closed { failure_counter } => {
                *failure_counter = 0;
                self.current_reset_timeout = self.reset_timeout;
            }
            State::HalfOpen => {}
            State::Open { open_time, current_reset_timeout } => {
                *open_time = SystemTime::now();
                let change_to_half_open = self.change_to_half_open.clone();
                self.scheduler.schedule_once(*current_reset_timeout, move || {
                    change_to_half_open();
                });
                let rnd = 1.0 * random::<f64>() * self.random_factor;
                let next_reset_timeout = (*current_reset_timeout)
                    .mul_f64(self.exponential_backoff_factor)
                    .mul_f64(rnd);
                if next_reset_timeout < self.max_reset_timeout {
                    *current_reset_timeout = next_reset_timeout;
                }
            }
        }
    }

    pub fn succeed(&mut self) {
        self.call_succeeds();
    }

    pub fn fail(&mut self) {
        self.call_fails();
    }

    pub fn on_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + Send + 'static {
        self.on_open_listeners.push(Box::new(f));
        self
    }

    pub fn on_half_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + Send + 'static {
        self.on_half_open_listeners.push(Box::new(f));
        self
    }

    pub fn on_close<F>(&mut self, f: F) -> &mut Self where F: Fn() + Send + 'static {
        self.on_close_listeners.push(Box::new(f));
        self
    }

    pub fn call_failure<F>(&mut self, f: F) -> &mut Self where F: Fn(SystemTime) + Send + 'static {
        self.call_failure_listeners.push(Box::new(f));
        self
    }

    pub fn call_timeout<F>(&mut self, f: F) -> &mut Self where F: Fn(SystemTime) + Send + 'static {
        self.call_timeout_listeners.push(Box::new(f));
        self
    }

    pub fn call_breaker_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + Send + 'static {
        self.call_breaker_open_listeners.push(Box::new(f));
        self
    }

    pub fn call_success<F>(&mut self, f: F) -> &mut Self where F: Fn() + Send + 'static {
        self.call_success_listeners.push(Box::new(f));
        self
    }

    pub fn change_to_half_open(&mut self) {
        self.transition(self.state, State::HalfOpen);
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum State {
    Closed {
        failure_counter: usize,
    },
    HalfOpen,
    Open {
        open_time: SystemTime,
        current_reset_timeout: Duration,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            State::Closed { .. } => {
                write!(f, "Closed")
            }
            State::HalfOpen => {
                write!(f, "HalfOpen")
            }
            State::Open { .. } => {
                write!(f, "Open")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use anyhow::anyhow;
    use async_trait::async_trait;
    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, Message};
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::props::Props;
    use crate::actor_ref::actor_ref_factory::ActorRefFactory;
    use crate::actor_ref::ActorRefExt;
    use crate::config::actor_setting::ActorSetting;
    use crate::pattern::circuit_breaker::CircuitBreaker;

    struct LogicActor {
        breaker: CircuitBreaker,
        counter: usize,
    }

    impl LogicActor {
        fn new_breaker(context: &mut ActorContext) -> anyhow::Result<CircuitBreaker> {
            let myself = context.myself().clone();
            let breaker = CircuitBreaker::new(
                context.system().scheduler().clone(),
                10,
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                1.0,
                0.2,
                move || {
                    myself.cast_ns(ChangeToHalfOpen);
                },
            );
            breaker
        }
    }

    #[async_trait]
    impl Actor for LogicActor {
        async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
            self.breaker.on_open(|| {
                info!("breaker now open");
            });
            self.breaker.on_close(|| {
                info!("breaker now close");
            });
            self.breaker.on_half_open(|| {
                info!("breaker now half open");
            });
            self.breaker.call_success(|| {
                info!("breaker call success");
            });
            self.breaker.call_failure(|_| {
                info!("breaker call failure");
            });
            self.breaker.call_timeout(|_| {
                info!("breaker call timeout");
            });
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct ChangeToHalfOpen;

    #[async_trait]
    impl Message for ChangeToHalfOpen {
        type A = LogicActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            actor.breaker.change_to_half_open();
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct SuccessCall(usize);

    #[async_trait]
    impl Message for SuccessCall {
        type A = LogicActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            for _ in 0..self.0 {
                let _ = actor.breaker.invoke(async {
                    actor.counter += 1;
                    Ok(())
                }).await;
            }
            info!("after success call, counter is {}", actor.counter);
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct FailCall(usize);

    #[async_trait]
    impl Message for FailCall {
        type A = LogicActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            for _ in 0..self.0 {
                let _ = actor.breaker.invoke::<_, ()>(async {
                    Err(anyhow!("test error"))
                }).await;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_breaker() -> anyhow::Result<()> {
        let system = ActorSystem::create("mikai233", ActorSetting::default())?;
        let actor = system.spawn_anonymous(Props::new_with_ctx(|ctx| {
            let breaker = LogicActor::new_breaker(ctx)?;
            Ok(LogicActor {
                breaker,
                counter: 0,
            })
        }))?;
        actor.cast_ns(SuccessCall(10));
        actor.cast_ns(FailCall(10));
        actor.cast_ns(SuccessCall(10));
        tokio::time::sleep(Duration::from_secs(4)).await;
        actor.cast_ns(SuccessCall(10));
        actor.cast_ns(FailCall(10));
        tokio::time::sleep(Duration::from_secs(4)).await;
        actor.cast_ns(FailCall(1));
        system.await?;
        Ok(())
    }
}