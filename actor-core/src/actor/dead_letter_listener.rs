use async_trait::async_trait;
use actor_derive::EmptyCodec;
use crate::{Actor, DynMessage, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::context::ActorContext;

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

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}