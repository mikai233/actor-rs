use crate::actor::props::Props;
use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::pattern::backoff_opts::BackoffReset;

pub(crate) trait HandBackoff {
    fn child_props(&self) -> Props;

    fn child_name(&self) -> &str;

    fn reset(&self) -> &BackoffReset;

    fn handle_message_to_child(&self, message: DynMessage);

    fn child(&self) -> Option<&ActorRef>;

    fn child_mut(&mut self) -> &mut Option<ActorRef>;

    fn restart_count(&self) -> usize;

    fn restart_count_mut(&mut self) -> &mut usize;

    fn final_stop_message_received(&self) -> bool;

    fn final_stop_message_received_mut(&mut self) -> &mut bool;
}
