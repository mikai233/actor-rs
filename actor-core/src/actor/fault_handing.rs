use std::ops::Deref;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::actor::actor_path::TActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::ActorContext;

pub trait SupervisorStrategy: Send + Sync + 'static {
    fn directive(&self) -> &Directive;
    fn handle_child_terminated(&self, context: &mut ActorContext, child: &ActorRef, children: Vec<ActorRef>) {}
    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef);
    fn handle_failure(&self, context: &mut ActorContext, child: &ActorRef) -> bool {
        let directive = self.directive();
        match directive {
            Directive::Resume => {
                self.resume_child(child);
                true
            }
            Directive::Restart => {
                self.process_failure(context, true, child);
                true
            }
            Directive::Stop => {
                self.process_failure(context, false, child);
                true
            }
            Directive::Escalate => {
                false
            }
        }
    }

    fn resume_child(&self, child: &ActorRef) {
        child.resume();
    }

    fn restart_child(&self, child: &ActorRef) {
        child.restart();
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub enum Directive {
    Resume,
    #[default]
    Restart,
    Stop,
    Escalate,
}

#[derive(Debug)]
pub struct ChildRestartStats {
    pub(crate) child: ActorRef,
    pub(crate) max_nr_of_retries_count: AtomicI32,
    pub(crate) restart_time_window_start_nanos: AtomicI64,
}

impl Deref for ChildRestartStats {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl ChildRestartStats {
    pub fn new(child: impl Into<ActorRef>) -> Self {
        Self {
            child: child.into(),
            max_nr_of_retries_count: AtomicI32::default(),
            restart_time_window_start_nanos: AtomicI64::default(),
        }
    }
    pub fn uid(&self) -> i32 {
        self.child.path().uid()
    }

    pub fn request_restart_permission(&self, retries_window: (Option<i32>, Option<i32>)) -> bool {
        match retries_window {
            (Some(retries), _) if retries < 1 => false,
            (Some(retries), None) => {
                self.max_nr_of_retries_count.fetch_add(1, Ordering::Relaxed) + 1 <= retries
            }
            (x, Some(window)) => self.retries_in_window_okay(x.unwrap_or(1), window),
            (None, _) => true,
        }
    }

    pub fn retries_in_window_okay(&self, retries: i32, window: i32) -> bool {
        let retries_done = self.max_nr_of_retries_count.load(Ordering::Relaxed) + 1;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
        let window_start = if self.restart_time_window_start_nanos.load(Ordering::Relaxed) == 0 {
            self.restart_time_window_start_nanos.store(now, Ordering::Relaxed);
            now
        } else {
            self.restart_time_window_start_nanos.load(Ordering::Relaxed)
        };
        let inside_window = (now - window_start) <= Duration::from_millis(window as u64).as_nanos() as i64;
        if inside_window {
            self.max_nr_of_retries_count.store(retries_done, Ordering::Relaxed);
            retries_done <= retries
        } else {
            self.max_nr_of_retries_count.store(1, Ordering::Relaxed);
            self.restart_time_window_start_nanos.store(now, Ordering::Relaxed);
            true
        }
    }
}

struct AllForOneStrategy {
    max_nr_of_retries: i32,
    within_time_range: Duration,
    directive: Directive,
}

impl SupervisorStrategy for AllForOneStrategy {
    fn directive(&self) -> &Directive {
        &self.directive
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef) {
        // if restart {
        //     for child_stats in children {
        //         // self.restart_child(&child_stats.child, error)
        //     }
        // } else {
        //     for child_stats in children {
        //         context.stop(&child_stats.child);
        //     }
        // }
    }
}

#[derive(Debug, Copy, Clone)]
struct OneForOneStrategy {
    max_nr_of_retries: i32,
    within_time_range: Option<Duration>,
    directive: Directive,
}

impl Default for OneForOneStrategy {
    fn default() -> Self {
        Self {
            max_nr_of_retries: -1,
            within_time_range: None,
            directive: Directive::default(),
        }
    }
}

impl SupervisorStrategy for OneForOneStrategy {
    fn directive(&self) -> &Directive {
        &self.directive
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef) {
        if restart && child.request_restart_permission(self.retries_window()) {
            self.restart_child(child);
        } else {
            context.stop(child);
        }
    }
}

impl OneForOneStrategy {
    fn retries_window(&self) -> (Option<i32>, Option<i32>) {
        (max_nr_or_retries_option(self.max_nr_of_retries), within_time_range_option(self.within_time_range).map(|r| r.as_millis() as i32))
    }
}

pub fn default_strategy() -> Box<dyn SupervisorStrategy> {
    Box::new(OneForOneStrategy::default())
}

pub fn stopping_strategy() -> Box<dyn SupervisorStrategy> {
    let mut default = OneForOneStrategy::default();
    default.directive = Directive::Stop;
    Box::new(default)
}

fn max_nr_or_retries_option(max_nr_or_retries: i32) -> Option<i32> {
    if max_nr_or_retries < 0 { None } else { Some(max_nr_or_retries) }
}

fn within_time_range_option(within_time_range: Option<Duration>) -> Option<Duration> {
    match within_time_range {
        Some(within_time_range) if within_time_range > Duration::ZERO => {
            Some(within_time_range)
        }
        _ => {
            None
        }
    }
}