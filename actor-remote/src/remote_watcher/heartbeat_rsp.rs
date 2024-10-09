use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::Context;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct HeartbeatRsp {
    address_uid: i64,
}

#[async_trait]
impl Message for HeartbeatRsp {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.receive_heartbeat_rsp(context, self.address_uid)
    }
}