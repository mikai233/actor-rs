use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_region::coordinator_terminated::CoordinatorTerminated;
use crate::shard_region::ShardRegion;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct RegisterAck {
    pub(crate) coordinator: ActorRef,
}

#[async_trait]
impl Message for RegisterAck {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if context.is_watching(&self.coordinator).not() {
            context.watch(self.coordinator.clone(), CoordinatorTerminated::new)?;
        }
        actor.coordinator = Some(self.coordinator);
        actor.finish_registration();
        actor.try_request_shard_buffer_homes(context);
        Ok(())
    }
}