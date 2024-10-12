use crate::actor::behavior::Behavior;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use crate::message::{DynMessage, Message};
use crate::routing::routee::TRoutee;
use crate::routing::router_actor::Router;
use crate::routing::router_config::TRouterConfig;
use actor_derive::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("RouteeEnvelope {{ message: {message} }}")]
pub struct RouteeEnvelope {
    pub message: DynMessage,
}

impl RouteeEnvelope {
    pub fn new<M>(message: M) -> Self
    where
        M: Message,
    {
        Self {
            message: Box::new(message),
        }
    }
}

impl<A: Router> MessageHandler<A> for RouteeEnvelope {
    fn handle(
        actor: &mut A,
        _: &mut A::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let routee = actor
            .router_config()
            .routing_logic()
            .select(&message.message, actor.routees());
        routee.send(message.message, sender);
        Ok(Behavior::same())
    }
}
