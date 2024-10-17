use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};
use serde::{Deserialize, Serialize};

use crate::remote_watcher::RemoteWatcher;

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("HeartbeatRsp {{ address_uid: {} }}", address_uid)]
pub(crate) struct HeartbeatRsp {
    address_uid: i64,
}

impl MessageHandler<RemoteWatcher> for HeartbeatRsp {
    fn handle(
        actor: &mut RemoteWatcher,
        ctx: &mut <RemoteWatcher as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        actor.receive_heartbeat_rsp(ctx, message.address_uid, sender)?;
        Ok(Behavior::same())
    }
}
