use super::Actor;
use crate::actor::behavior::Behavior;
use crate::actor_ref::ActorRef;
use crate::cell::actor_cell::ActorCell;
use crate::message::handler::MessageHandler;
use crate::message::DynMessage;
use actor_derive::Message;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct DeadLetterListener;

impl Actor for DeadLetterListener {
    type Context = ActorCell;

    fn receive(&self) -> super::receive::Receive<Self> {
        todo!()
    }
}

#[derive(Debug, Message)]
pub struct Dropped {
    message: DynMessage,
    reason: String,
    sender: Option<ActorRef>,
}

impl Dropped {
    pub fn new(message: DynMessage, reason: String, sender: Option<ActorRef>) -> Self {
        Self {
            message,
            reason,
            sender,
        }
    }
}

impl Display for Dropped {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dropped {{ message: {}, reason: {}, sender: {:?} }}",
            self.message,
            self.reason,
            self.sender,
        )
    }
}

impl MessageHandler<DeadLetterListener> for Dropped {
    fn handle(
        actor: &mut DeadLetterListener,
        ctx: &mut <DeadLetterListener as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<DeadLetterListener>> {
        todo!()
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("DeadMessage({_0})")]
pub struct DeadMessage(pub DynMessage);

impl MessageHandler<DeadLetterListener> for DeadMessage {
    fn handle(
        actor: &mut DeadLetterListener,
        ctx: &mut <DeadLetterListener as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<DeadLetterListener>> {
        todo!()
    }
}