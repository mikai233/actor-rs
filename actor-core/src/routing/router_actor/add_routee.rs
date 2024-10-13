use std::fmt::Display;

use actor_derive::Message;

use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;

#[derive(Debug, Message)]
pub struct AddRoutee {
    pub routee: Routee,
}

impl Display for AddRoutee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AddRoutee {{ {} }}", self.routee)
    }
}

impl<R: Router> MessageHandler<R> for AddRoutee {
    fn handle(
        actor: &mut R,
        ctx: &mut <R as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<R>,
    ) -> anyhow::Result<Behavior<R>> {
        if let Routee::ActorRefRoutee(routee) = &message.routee {
            ctx.context_mut().watch(routee)?;
        }
        actor.routees_mut().push(message.routee);
        Ok(Behavior::same())
    }
}
