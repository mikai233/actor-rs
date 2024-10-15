use crate::actor::behavior::Behavior;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use crate::message::{DynMessage, Message};
use crate::routing::routee::TRoutee;
use crate::routing::router_actor::Router;
use actor_derive::Message;
use anyhow::anyhow;

#[derive(Debug, Message, derive_more::Display)]
#[display("Broadcast {{ message: {message} }}")]
#[cloneable]
pub struct Broadcast {
    message: DynMessage,
}

impl Broadcast {
    pub fn new<M>(message: M) -> Self
    where
        M: Message + Clone,
    {
        Self {
            message: Box::new(message),
        }
    }

    pub fn message(&self) -> &DynMessage {
        &self.message
    }
}

impl<A: Router> MessageHandler<A> for Broadcast {
    fn handle(
        actor: &mut A,
        _: &mut A::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        for routee in actor.routees() {
            let message = message.message.clone_box().ok_or(anyhow!(
                "message {} is not cloneable",
                message.message.signature()
            ))?;
            routee.send(message, sender.clone());
        }
        Ok(Behavior::same())
    }
}

impl Into<DynMessage> for Broadcast {
    fn into(self) -> DynMessage {
        self.message
    }
}

impl Clone for Broadcast {
    fn clone(&self) -> Self {
        Self {
            message: self
                .message
                .clone_box()
                .expect("Broadcast message is not cloneable"),
        }
    }
}
