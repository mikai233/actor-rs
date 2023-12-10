use std::time::Duration;
use anyhow::Error;
use crate::actor::actor_path::TActorPath;
use crate::actor::actor_ref::ActorRef;
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
    fn handle_child_terminated(&self, context: &mut ActorContext, child: &ActorRef, children: Vec<ActorRef>);
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

    fn resume_child(&self, child: &ActorRef, error: anyhow::Error) {
        todo!()
    }

    fn restart_child(&self, child: &ActorRef, error: anyhow::Error) {
        todo!()
    }

    fn log_failure(&self, context: &mut ActorContext, child: &ActorRef, error: &anyhow::Error, decision: Directive) {
        todo!()
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
    child: ActorRef,
    max_nr_of_retries_count: i32,
    restart_time_window_start_nanos: i64,
}

impl ChildRestartStats {
    pub fn uid(&self) -> i32 {
        self.child.path().uid()
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

    fn handle_child_terminated(&self, context: &mut ActorContext, child: &ActorRef, children: Vec<ActorRef>) {
        todo!()
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef, error: Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>) {
        todo!()
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

    fn handle_child_terminated(&self, context: &mut ActorContext, child: &ActorRef, children: Vec<ActorRef>) {
        todo!()
    }

    fn process_failure(&self, context: &mut ActorContext, restart: bool, child: &ActorRef, error: Error, stats: ChildRestartStats, children: Vec<ChildRestartStats>) {
        todo!()
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