use async_trait::async_trait;

use actor_core::{EmptyCodec, Message, OrphanEmptyCodec};
use actor_core::actor::context::ActorContext;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, EmptyCodec)]
pub struct Count;

#[async_trait]
impl Message for Count {
    type A = DistributedPubSubMediator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub struct CountSubscribers {
    pub topic: String,
}