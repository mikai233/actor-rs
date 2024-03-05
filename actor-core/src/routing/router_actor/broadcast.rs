use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message};
use crate::actor::context::{ActorContext, Context};
use crate::routing::routee::TRoutee;
use crate::routing::router_actor::Router;

#[derive(Debug, EmptyCodec)]
pub struct Broadcast {
    message: DynMessage,
}

#[async_trait]
impl Message for Broadcast {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        for routee in actor.routees() {
            routee.send(self.message.dyn_clone()?, context.sender().cloned());
        }
        Ok(())
    }
}