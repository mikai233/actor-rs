use std::any::{Any, type_name};
use std::time::Duration;

use async_trait::async_trait;

use actor_core::{Actor, CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::message::codec::MessageRegistry;
use actor_core::message::MessageDecoder;
use actor_core::routing::routing_logic::RoutingLogic;

use crate::pubsub::distributed_pub_sub_mediator::subscribe::Subscribe;

pub(crate) struct Topic {
    empty_time_to_live: Duration,
    routing_logic: Box<dyn RoutingLogic>,
}

#[async_trait]
impl Actor for Topic {
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

impl CodecMessage for Subscribe<Topic> {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(self: Box<Self>, _reg: &MessageRegistry) -> anyhow::Result<Vec<u8>> {
        Err(anyhow::anyhow!("{} cannot codec", type_name::<Subscribe<Topic>>()))
    }

    fn clone_box(&self) -> anyhow::Result<Box<dyn CodecMessage>> {
        Err(anyhow::anyhow!("message {} is not cloneable", type_name::<Subscribe<Topic>>()))
    }

    fn cloneable(&self) -> bool {
        false
    }

    fn into_dyn(self) -> DynMessage {
        DynMessage::user(self)
    }
}

#[async_trait]
impl Message for Subscribe<Topic> {
    type A = Topic;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        Ok(())
    }
}