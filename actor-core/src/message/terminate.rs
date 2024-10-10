use actor_derive::Message;
use serde::{Deserialize, Serialize};

use crate::{
    actor::{behavior::Behavior, context::ActorContext, Actor},
    actor_ref::ActorRef,
};

use super::handler::MessageHandler;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[display("Terminate")]
pub struct Terminate;

impl<A: Actor> MessageHandler<A> for Terminate {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>> {
        ctx.context_mut().terminate();
        Ok(Behavior::same())
    }
}
