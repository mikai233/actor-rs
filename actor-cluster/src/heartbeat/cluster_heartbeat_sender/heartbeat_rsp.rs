use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::unique_address::UniqueAddress;

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("HeartbeatRsp {{ from: {from} }}")]
pub(crate) struct HeartbeatRsp {
    pub(crate) from: UniqueAddress,
}

impl MessageHandler<ClusterHeartbeatSender> for HeartbeatRsp {
    fn handle(
        _: &mut ClusterHeartbeatSender,
        ctx: &mut <ClusterHeartbeatSender as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ClusterHeartbeatSender>,
    ) -> anyhow::Result<Behavior<ClusterHeartbeatSender>> {
        trace!("{} recv HeartbeatRsp from {}", ctx.myself(), message.from);
        Ok(Behavior::same())
    }
}
