use std::any::type_name;
use std::ops::Not;

use eyre::anyhow;
use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message};
use crate::actor::context::{ActorContext, Context};
use crate::routing::router_actor::Router;
use crate::routing::routee::TRoutee;

#[derive(Debug, EmptyCodec)]
pub struct Broadcast {
    message: DynMessage,
}

impl Broadcast {
    pub fn new<M>(message: M) -> eyre::Result<Self> where M: Message {
        if message.is_cloneable().not() {
            return Err(anyhow!("broadcast message {} require cloneable", type_name::<M>()));
        }
        let msg = Self {
            message: DynMessage::user(message),
        };
        Ok(msg)
    }
}

#[async_trait]
impl Message for Broadcast {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        for routee in actor.routees() {
            routee.send(self.message.dyn_clone()?, context.sender().cloned());
        }
        Ok(())
    }
}