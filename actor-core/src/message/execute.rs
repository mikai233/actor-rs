use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::context::ActorContext;

#[derive(EmptyCodec)]
pub struct Execute<A> where A: Actor {
    pub closure: Box<dyn FnOnce(&mut ActorContext, &mut A) -> anyhow::Result<()> + Send>,
}

#[async_trait]
impl<A> Message for Execute<A> where A: Actor {
    type A = A;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        (self.closure)(context, actor)
    }
}