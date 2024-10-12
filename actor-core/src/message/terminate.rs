use actor_derive::Message;
use serde::{Deserialize, Serialize};

use crate::{
    actor::{behavior::Behavior, context::ActorContext, receive::Receive, Actor},
    actor_ref::ActorRef,
};

use super::handler::MessageHandler;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[display("Terminate")]
pub struct Terminate;

impl<A: Actor> MessageHandler<A> for Terminate {
    fn handle(
        _: &mut A,
        ctx: &mut <A as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        ctx.context_mut().terminate();
        Ok(Behavior::same())
    }
}
