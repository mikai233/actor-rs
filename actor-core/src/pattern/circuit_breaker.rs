use std::fmt::{Display, Formatter};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use rand::random;

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
    on_open_listeners: Vec<Box<dyn Fn()>>,
    on_half_open_listeners: Vec<Box<dyn Fn()>>,
    on_close_listeners: Vec<Box<dyn Fn()>>,
    call_failure_listeners: Vec<Box<dyn Fn(SystemTime)>>,
    call_timeout_listeners: Vec<Box<dyn Fn(SystemTime)>>,
    call_breaker_open_listeners: Vec<Box<dyn Fn()>>,
    call_success_listeners: Vec<Box<dyn Fn()>>,
    change_to_half_open: Arc<Box<dyn Fn() + Send + Sync>>,
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

    fn transition_listeners(&self, state: State) -> &Vec<Box<dyn Fn()>> {
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

    fn transition_listeners_mut(&mut self, state: State) -> &mut Vec<Box<dyn Fn()>> {
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
                if current_reset_timeout >= duration_from_opened {
                    Err(anyhow!("breaker is open, will change to half open immdeately"))
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
        match &mut self.state {
            State::Closed { failure_counter } => {
                *failure_counter += 1;
                if *failure_counter >= self.max_failures {
                    self.transition(
                        self.state.clone(),
                        State::Open {
                            open_time: SystemTime::now(),
                            current_reset_timeout: Duration::ZERO,
                        },
                    );
                }
            }
            State::HalfOpen => {
                self.transition(
                    self.state.clone(),
                    State::Open {
                        open_time: SystemTime::now(),
                        current_reset_timeout: Duration::ZERO,
                    },
                );
            }
            State::Open { .. } => {}
        }
    }

    fn transition(&mut self, from_state: State, to_state: State) {
        if from_state == to_state {
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

    pub fn on_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + 'static {
        self.on_open_listeners.push(Box::new(f));
        self
    }

    pub fn on_half_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + 'static {
        self.on_half_open_listeners.push(Box::new(f));
        self
    }

    pub fn on_close<F>(&mut self, f: F) -> &mut Self where F: Fn() + 'static {
        self.on_close_listeners.push(Box::new(f));
        self
    }

    pub fn call_failure<F>(&mut self, f: F) -> &mut Self where F: Fn(SystemTime) + 'static {
        self.call_failure_listeners.push(Box::new(f));
        self
    }

    pub fn call_timeout<F>(&mut self, f: F) -> &mut Self where F: Fn(SystemTime) + 'static {
        self.call_timeout_listeners.push(Box::new(f));
        self
    }

    pub fn call_breaker_open<F>(&mut self, f: F) -> &mut Self where F: Fn() + 'static {
        self.call_breaker_open_listeners.push(Box::new(f));
        self
    }

    pub fn call_success<F>(&mut self, f: F) -> &mut Self where F: Fn() + 'static {
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