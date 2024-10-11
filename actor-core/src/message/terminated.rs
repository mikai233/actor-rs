use crate::{
    actor::{behavior::Behavior, context::ActorContext, receive::Receive, Actor},
    actor_ref::ActorRef,
};
use actor_derive::Message;
use std::fmt::{Display, Formatter};

use super::handler::MessageHandler;

#[derive(Debug, Clone, Message, derive_more::Deref)]
#[cloneable]
pub struct Terminated {
    #[deref]
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}

impl Terminated {
    pub fn new(watchee: ActorRef) -> Self {
        Self {
            actor: watchee,
            existence_confirmed: true,
            address_terminated: false,
        }
    }
}

impl Display for Terminated {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Terminated {{actor: {}, existence_confirmed: {}, address_terminated: {} }}",
            self.actor, self.existence_confirmed, self.address_terminated,
        )
    }
}

impl<A: Actor> MessageHandler<A> for Terminated {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        receive: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let context = ctx.context_mut();
        if let Some(optional_message) = context.terminated_queue.remove(&message.actor) {
            match optional_message {
                Some(custom_termination) => {
                    actor.around_receive(receive, actor, ctx, message, sender)
                    //TODO currentMessage?
                }
                None => actor.around_receive(receive, actor, ctx, message, sender),
            }
        } else {
            Ok(Behavior::same())
        }
    }
}
