use crate::handoff_stopper::{HandoffStopper, STOP_TIMEOUT_WARNING_AFTER};
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::coordinated_shutdown::CoordinatedShutdown;
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::warn;

#[derive(Debug, Message, derive_more::Display)]
#[display("StopTimeoutWarning")]
pub(super) struct StopTimeoutWarning;

impl MessageHandler<HandoffStopper> for StopTimeoutWarning {
    fn handle(
        actor: &mut HandoffStopper,
        ctx: &mut HandoffStopper::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<HandoffStopper>,
    ) -> anyhow::Result<Behavior<HandoffStopper>> {
        let is_terminating = CoordinatedShutdown::get(ctx.system()).is_terminating();
        let type_name = &actor.type_name;
        let remaining_size = actor.remaining_entities.len();
        let shard = &actor.shard;
        let timeout = &STOP_TIMEOUT_WARNING_AFTER;
        let stop_msg = actor.stop_message.signature();
        if is_terminating {
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled?");
        } else {
            let remaining_timeout = actor.entity_handoff_timeout - *timeout;
            warn!( "{type_name}: [{remaining_size}] of the entities in shard [{shard}] not stopped after [{timeout:?}]. Maybe the handoff stop message [{stop_msg}] is not handled? \
            Waiting additional [{remaining_timeout:?}] before stopping the remaining entities.");
        }
        Ok(Behavior::same())
    }
}