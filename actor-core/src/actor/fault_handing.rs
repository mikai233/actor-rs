use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dyn_clone::DynClone;

use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};

pub trait SupervisorStrategy: Send + Sync + DynClone + 'static {
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

dyn_clone::clone_trait_object!(SupervisorStrategy);

#[derive(Debug, Copy, Clone, Default)]
pub enum Directive {
    Resume,
    #[default]
    Restart,
    Stop,
    Escalate,
}

#[derive(Debug, Default)]
pub struct ChildRestartStats {
    pub(crate) max_nr_of_retries_count: i32,
    pub(crate) restart_time_window_start_nanos: i64,
}

impl ChildRestartStats {
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
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
        let window_start = if self.restart_time_window_start_nanos == 0 {
            self.restart_time_window_start_nanos = now;
            now
        } else {
            self.restart_time_window_start_nanos
        };
        let inside_window = (now - window_start) <= Duration::from_millis(window as u64).as_nanos() as i64;
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

#[derive(Debug, Copy, Clone)]
pub struct AllForOneStrategy {
    pub max_nr_of_retries: i32,
    pub within_time_range: Option<Duration>,
    pub directive: Directive,
}

impl SupervisorStrategy for AllForOneStrategy {
    fn directive(&self) -> &Directive {
        &self.directive
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef) {
        let children = context.children();
        let myself = context.myself.local().unwrap().cell.restart_stats();
        if !children.is_empty() {
            if restart && children.iter().all(|child| myself.get_mut(child).unwrap().value_mut().request_restart_permission(self.retries_window())) {
                for child in children {
                    self.restart_child(&child);
                }
            } else {
                for child in children {
                    context.stop(&child);
                }
            }
        }
    }
}

impl AllForOneStrategy {
    fn retries_window(&self) -> (Option<i32>, Option<i32>) {
        (max_nr_or_retries_option(self.max_nr_of_retries), within_time_range_option(self.within_time_range).map(|r| r.as_millis() as i32))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct OneForOneStrategy {
    pub max_nr_of_retries: i32,
    pub within_time_range: Option<Duration>,
    pub directive: Directive,
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
        let myself = context.myself.local().unwrap();
        let restart_stats = myself.cell.restart_stats();
        let mut stats = restart_stats.get_mut(child).unwrap();
        if restart && stats.value_mut().request_restart_permission(self.retries_window()) {
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