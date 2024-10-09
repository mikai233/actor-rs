use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message};
use crate::actor::context::{Context, ActorContext};
use crate::routing::routee::TRoutee;
use crate::routing::router_actor::Router;
use crate::routing::router_config::TRouterConfig;

#[derive(Debug, EmptyCodec)]
pub struct RouteeEnvelope {
    pub message: DynMessage,
}

impl RouteeEnvelope {
    pub fn new<M>(message: M) -> Self where M: Message {
        Self {
            message: DynMessage::user(message),
        }
    }
}

#[async_trait]
impl Message for RouteeEnvelope {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let routee = actor.router_config().routing_logic().select(&self.message, actor.routees());
        routee.send(self.message, context.sender().cloned());
        Ok(())
    }
}