use super::{context::Context, receive::Receive, Actor};
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use crate::message::DynMessage;
use crate::{actor::behavior::Behavior, message::Message};
use actor_derive::Message;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct DeadLetterListener;

impl Actor for DeadLetterListener {
    type Context = Context;

    fn receive(&self) -> Receive<Self> {
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
    pub fn new<M>(message: M, reason: String, sender: Option<ActorRef>) -> Self
    where
        M: Message,
    {
        Self {
            message: Box::new(message),
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
            self.message, self.reason, self.sender,
        )
    }
}

impl MessageHandler<DeadLetterListener> for Dropped {
    fn handle(
        _: &mut DeadLetterListener,
        _: &mut <DeadLetterListener as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<DeadLetterListener>,
    ) -> anyhow::Result<Behavior<DeadLetterListener>> {
        todo!()
    }
}

#[derive(Debug, Message, derive_more::Display)]
#[display("DeadMessage({_0})")]
pub struct DeadMessage(pub DynMessage);

impl MessageHandler<DeadLetterListener> for DeadMessage {
    fn handle(
        _: &mut DeadLetterListener,
        _: &mut <DeadLetterListener as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<DeadLetterListener>,
    ) -> anyhow::Result<Behavior<DeadLetterListener>> {
        todo!()
    }
}
