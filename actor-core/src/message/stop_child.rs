use std::marker::PhantomData;
use async_trait::async_trait;
use actor_derive::EmptyCodec;
use crate::{Actor, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::ActorContext;

#[derive(EmptyCodec)]
pub(crate) struct StopChild<A: Actor> {
    _phantom: PhantomData<A>,
    child: ActorRef,
}

impl<A> StopChild<A> where A: Actor {
    pub fn new(child: ActorRef) -> Self {
        Self {
            _phantom: PhantomData::default(),
            child,
        }
    }
}

#[async_trait]
impl<T> Message for StopChild<T> where T: Actor {
    type A = T;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.stop(&self.child);
        Ok(())
    }
}