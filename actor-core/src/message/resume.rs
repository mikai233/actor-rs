use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;
use crate::actor::Actor;
use crate::actor::{behavior::Behavior, receive::Receive};
use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};
use tracing::trace;

use super::handler::MessageHandler;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("Resume")]
pub struct Resume;

impl<A: Actor> MessageHandler<A> for Resume {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let context = ctx.context_mut();
        debug_assert!(matches!(context.state, ActorState::Suspend));
        context.state = ActorState::Started;
        for child in context.children() {
            child.resume();
        }
        trace!("{} resume", context.myself);
        Ok(Behavior::same())
    }
}
