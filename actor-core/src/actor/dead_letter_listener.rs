use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{Actor, DynMessage, Message};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug)]
pub struct DeadLetterListener;

#[async_trait]
impl Actor for DeadLetterListener {
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

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

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        //TODO
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub struct DeadMessage(pub DynMessage);

#[async_trait]
impl Message for DeadMessage {
    type A = DeadLetterListener;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        //TODO
        Ok(())
    }
}