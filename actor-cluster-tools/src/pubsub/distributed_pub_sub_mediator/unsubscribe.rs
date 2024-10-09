use async_trait::async_trait;

use actor_core::{EmptyCodec, Message};
use actor_core::actor::context::Context;
use actor_core::actor_ref::ActorRef;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, EmptyCodec)]
pub struct Unsubscribe {
    pub topic: String,
    pub subscriber: ActorRef,
}

#[async_trait]
impl Message for Unsubscribe {
    type A = DistributedPubSubMediator;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}