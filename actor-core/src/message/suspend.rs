use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use actor_derive::Message;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("Suspend")]
pub struct Suspend;

impl<A: Actor> MessageHandler<A> for Suspend {
    fn handle(
        actor: &mut A,
        ctx: &mut A::Context,
        message: Self,
        _: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>> {
        let context = ctx.context_mut();
        context.state = ActorState::Suspend;
        trace!("{} suspend", context.myself);
        Ok(Behavior::same())
    }
}
