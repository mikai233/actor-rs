use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message};
use crate::actor::context::ActorContext;
use crate::routing::router_actor::RouterActor;

#[derive(Debug, EmptyCodec)]
pub struct RouteeEnvelope  {
    message: DynMessage,
}

#[async_trait]
impl Message for RouteeEnvelope {
    type A = RouterActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}