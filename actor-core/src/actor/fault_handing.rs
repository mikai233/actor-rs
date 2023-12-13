use std::ops::Deref;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Error;

use crate::actor::actor_path::TActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::ActorContext;

pub type Decider = Box<dyn Fn(&anyhow::Error) -> Directive + Send + Sync + 'static>;

pub fn default_decider() -> Decider {
    Box::new(|_| Directive::Restart)
}

pub fn stopping_decider() -> Decider {
    Box::new(|_| Directive::Stop)
}

pub trait SupervisorStrategy: Send + Sync + 'static {
    fn decider(&self) -> &Decider;
    fn handle_child_terminated(&self, context: &mut ActorContext, child: &ActorRef, children: Vec<ActorRef>) {}
    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef, error: anyhow::Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>);
    fn handle_failure(&self, context: &mut ActorContext, child: &ActorRef, error: anyhow::Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>) -> bool {
        let directive = self.decider()(&error);
        match directive {
            Directive::Resume => {
                self.resume_child(child, error);
                true
            }
            Directive::Restart => {
                self.log_failure(context, child, &error, directive);
                self.process_failure(context, true, child, error, stats, children);
                true
            }
            Directive::Stop => {
                self.log_failure(context, child, &error, directive);
                self.process_failure(context, false, child, error, stats, children);
                true
            }
            Directive::Escalate => {
                self.log_failure(context, child, &error, directive);
                false
            }
        }
    }

    fn resume_child(&self, child: &ActorRef, error: Error) {
        child.resume(Some(error.to_string()));
    }

    fn restart_child(&self, child: &ActorRef, error: Error) {
        child.restart(Some(error.to_string()));
    }

    fn log_failure(&self, context: &mut ActorContext, child: &ActorRef, error: &Error, decision: Directive) {
        //TODO
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Directive {
    Resume,
    Restart,
    Stop,
    Escalate,
}

#[derive(Debug, Clone)]
pub struct ChildRestartStats {
    pub(crate) child: ActorRef,
    pub(crate) max_nr_of_retries_count: i32,
    pub(crate) restart_time_window_start_nanos: u128,
}

impl Deref for ChildRestartStats {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl ChildRestartStats {
    pub fn uid(&self) -> i32 {
        self.child.path().uid()
    }

    pub fn request_restart_permission(&mut self, retries_window: (Option<i32>, Option<i32>)) -> bool {
        match retries_window {
            (Some(retries), _) if retries < 1 => false,
            (Some(retries), None) => {
                self.max_nr_of_retries_count += 1;
                self.max_nr_of_retries_count <= retries
            }
            (x, Some(window)) => self.retries_in_window_okay(x.unwrap_or(1), window),
            (None, _) => true,
        }
    }

    pub fn retries_in_window_okay(&mut self, retries: i32, window: i32) -> bool {
        let retries_done = self.max_nr_of_retries_count + 1;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let window_start = if self.restart_time_window_start_nanos == 0 {
            self.restart_time_window_start_nanos = now;
            now
        } else {
            self.restart_time_window_start_nanos
        };
        let inside_window = (now - window_start) <= Duration::from_millis(window as u64).as_nanos();
        if inside_window {
            self.max_nr_of_retries_count = retries_done;
            retries_done <= retries
        } else {
            self.max_nr_of_retries_count = 1;
            self.restart_time_window_start_nanos = now;
            true
        }
    }
}

struct AllForOneStrategy {
    max_nr_of_retries: i32,
    within_time_range: Duration,
    decider: Decider,
}

impl SupervisorStrategy for AllForOneStrategy {
    fn decider(&self) -> &Decider {
        &self.decider
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef, error: Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>) {
        if restart {
            for child_stats in children {
                // self.restart_child(&child_stats.child, error)
            }
        } else {
            for child_stats in children {
                context.stop(&child_stats.child);
            }
        }
    }
}

struct OneForOneStrategy {
    max_nr_of_retries: i32,
    within_time_range: Duration,
    decider: Decider,
}

impl Default for OneForOneStrategy {
    fn default() -> Self {
        Self {
            max_nr_of_retries: -1,
            within_time_range: Duration::MAX,
            decider: default_decider(),
        }
    }
}

impl SupervisorStrategy for OneForOneStrategy {
    fn decider(&self) -> &Decider {
        &self.decider
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef, error: Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>) {
        if restart {
            self.restart_child(child, error);
        } else {
            context.stop(child);
        }
    }
}

pub fn default_strategy() -> Box<dyn SupervisorStrategy> {
    Box::new(OneForOneStrategy::default())
}

pub fn stopping_strategy() -> Box<dyn SupervisorStrategy> {
    let mut default = OneForOneStrategy::default();
    default.decider = stopping_decider();
    Box::new(default)
}