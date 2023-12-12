use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::context::ActorContext;

#[derive(EmptyCodec)]
pub struct Execute<A> where A: Actor {
    pub closure: Box<dyn FnOnce(&mut ActorContext, &mut A) -> anyhow::Result<()> + Send>,
}

impl<A> Message for Execute<A> where A: Actor {
    type A = A;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        (self.closure)(context, actor)
    }
}