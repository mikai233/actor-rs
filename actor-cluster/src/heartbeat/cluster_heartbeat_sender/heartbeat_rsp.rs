use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct HeartbeatRsp {
    pub(crate) from: UniqueAddress,
}

#[async_trait]
impl Message for HeartbeatRsp {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} recv HeartbeatRsp from {}", context.myself(), self.from);
        Ok(())
    }
}