use std::time::Duration;

use crate::actor::context::Context;
use crate::actor::props::{Props, PropsBuilder};
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::pattern::backoff_opts::{BackoffReset, HandlingWhileStopped};
use crate::pattern::hand_backoff::HandBackoff;

pub(crate) struct BackoffOnStopSupervisor {
    child_props: PropsBuilder<()>,
    child_name: String,
    min_backoff: Duration,
    max_backoff: Duration,
    reset: BackoffReset,
    random_factor: f64,
    handling_while_stopped: HandlingWhileStopped,
    final_stop_message: Option<Box<dyn Fn(DynMessage) -> bool + Send>>,
    child: Option<ActorRef>,
    restart_count: usize,
    final_stop_message_received: bool,
}

impl HandBackoff for BackoffOnStopSupervisor {
    fn child_props(&self) -> Props {
        self.child_props.props(())
    }

    fn child_name(&self) -> &str {
        &self.child_name
    }

    fn reset(&self) -> &BackoffReset {
        &self.reset
    }

    #[allow(unused_variables)]
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

impl Actor for BackoffOnStopSupervisor {
    type Context = Context;

    fn receive(&self) -> Receive<Self> {
        todo!()
    }
}
