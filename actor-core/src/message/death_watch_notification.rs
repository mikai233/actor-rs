use crate::{
    actor::{behavior::Behavior, context::ActorContext, receive::Receive, Actor},
    actor_ref::ActorRef,
};
use actor_derive::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

use super::handler::MessageHandler;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[cloneable]
pub struct DeathWatchNotification {
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}

impl Display for DeathWatchNotification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeathWatchNotification {{ actor: {}, existence_confirmed: {}, address_terminated: {} }}",
            self.actor,
            self.existence_confirmed,
            self.address_terminated,
        )
    }
}

impl<A: Actor> MessageHandler<A> for DeathWatchNotification {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let DeathWatchNotification {
            actor,
            existence_confirmed,
            address_terminated,
        } = message;
        let context = ctx.context_mut();
        context.watched_actor_terminated(actor, existence_confirmed, address_terminated);
        Ok(Behavior::same())
    }
}
