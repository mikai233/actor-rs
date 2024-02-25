use std::time::Duration;

use async_trait::async_trait;

use crate::{Actor, DynMessage};
use crate::actor::actor_ref::ActorRef;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::pattern::backoff_opts::{BackoffReset, HandlingWhileStopped};
use crate::pattern::hand_backoff::HandBackoff;

pub(crate) struct BackoffOnStopSupervisor {
    child_props: Props,
    child_name: String,
    min_backoff: Duration,
    max_backoff: Duration,
    reset: BackoffReset,
    random_factor: f64,
    strategy: Box<dyn SupervisorStrategy>,
    handling_while_stopped: HandlingWhileStopped,
    final_stop_message: Option<Box<dyn Fn(DynMessage) -> bool + Send>>,
    child: Option<ActorRef>,
    restart_count: usize,
    final_stop_message_received: bool,
}

impl HandBackoff for BackoffOnStopSupervisor {
    fn child_props(&self) -> Props {
        self.child_props.clone()
    }

    fn child_name(&self) -> &str {
        &self.child_name
    }

    fn reset(&self) -> &BackoffReset {
        &self.reset
    }

    fn handle_message_to_child(&self, message: DynMessage) {
        todo!()
    }

    fn child(&self) -> Option<&ActorRef> {
        self.child.as_ref()
    }

    fn child_mut(&mut self) -> &mut Option<ActorRef> {
        &mut self.child
    }

    fn restart_count(&self) -> usize {
        self.restart_count
    }

    fn restart_count_mut(&mut self) -> &mut usize {
        &mut self.restart_count
    }

    fn final_stop_message_received(&self) -> bool {
        self.final_stop_message_received
    }

    fn final_stop_message_received_mut(&mut self) -> &mut bool {
        &mut self.final_stop_message_received
    }
}

#[async_trait]
impl Actor for BackoffOnStopSupervisor {}