use actor_derive::Message;
use serde::{Deserialize, Serialize};

use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_ref::{ActorRef, ActorRefExt};

use super::handler::MessageHandler;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Message, derive_more::Display)]
#[cloneable]
#[display("Identify")]
pub struct Identify;

impl<A: Actor> MessageHandler<A> for Identify {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let context = ctx.context();
        let myself = context.myself.clone();
        let actor_identify = ActorIdentity {
            actor_ref: Some(myself),
        };
        if let Some(parent) = context.parent() {
            parent.cast_ns(actor_identify);
        }
        Ok(Behavior::same())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message, derive_more::Display)]
#[cloneable]
#[display("ActorIdentity {{ actor_ref: {actor_ref} }}")]
pub struct ActorIdentity {
    pub actor_ref: Option<ActorRef>,
}
