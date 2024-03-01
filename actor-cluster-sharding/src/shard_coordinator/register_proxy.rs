use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct RegisterProxy {
    pub(crate) shard_region_proxy: ActorRef,
}

#[async_trait]
impl Message for RegisterProxy {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}