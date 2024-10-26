use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_rsp::HeartbeatRsp;
use crate::member::MemberStatus;
use crate::unique_address::UniqueAddress;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("Heartbeat {{ from: {from} }}")]
#[cloneable]
pub(crate) struct Heartbeat {
    pub(crate) from: UniqueAddress,
}

impl MessageHandler<ClusterHeartbeatReceiver> for Heartbeat {
    fn handle(
        actor: &mut ClusterHeartbeatReceiver,
        ctx: &mut <ClusterHeartbeatReceiver as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterHeartbeatReceiver>,
    ) -> anyhow::Result<Behavior<ClusterHeartbeatReceiver>> {
        trace!("{} recv Heartbeat from {}", ctx.myself(), message.from);
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                let resp = HeartbeatRsp {
                    from: self_member.unique_address.clone(),
                };
                let sender = sender.ok_or(anyhow!("Sender is None"))?;
                sender.cast_ns(resp);
            }
        }
        Ok(Behavior::same())
    }
}
