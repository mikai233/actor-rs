use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;
use actor_derive::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("RemoveRoutee {{ routee: {routee} }}")]
pub struct RemoveRoutee {
    pub routee: Routee,
}


impl<A: Router> MessageHandler<A> for RemoveRoutee {
    fn handle(
        actor: &mut A,
        ctx: &mut A::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        actor.routees_mut().retain(|routee| *routee != message.routee);
        if let Routee::ActorRefRoutee(routee) = &message.routee {
            ctx.context_mut().unwatch(routee);
        }
        Ok(Behavior::same())
    }
}