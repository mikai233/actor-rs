use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::context::ActorContext;

#[derive(EmptyCodec)]
pub struct Execute<A> where A: Actor {
    pub closure: Box<dyn FnOnce(&mut ActorContext, &mut A) -> eyre::Result<()> + Send>,
}

#[async_trait]
impl<A> Message for Execute<A> where A: Actor {
    type A = A;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        (self.closure)(context, actor)
    }
}