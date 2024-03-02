use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_core::{DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::{ActorContext, Context};
use actor_derive::CMessageCodec;

use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_rsp::HeartbeatRsp;
use crate::member::MemberStatus;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct Heartbeat {
    pub(crate) from: UniqueAddress,
}

#[async_trait]
impl Message for Heartbeat {
    type A = ClusterHeartbeatReceiver;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} recv Heartbeat from {}", context.myself(), self.from);
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                context.sender().unwrap().tell(DynMessage::user(HeartbeatRsp { from: self_member.addr.clone() }), ActorRef::no_sender());
            }
        }
        Ok(())
    }
}