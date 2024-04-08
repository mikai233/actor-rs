use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{Actor, DynMessage, Message};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug)]
pub struct DeadLetterListener;

impl Actor for DeadLetterListener {}

#[derive(Debug, EmptyCodec)]
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

#[async_trait]
impl Message for Dropped {
    type A = DeadLetterListener;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        //TODO
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub struct DeadMessage(pub DynMessage);

#[async_trait]
impl Message for DeadMessage {
    type A = DeadLetterListener;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        //TODO
        Ok(())
    }
}