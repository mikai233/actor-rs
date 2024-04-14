use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ArteryHeartbeatRsp {
    pub(super) uid: i64,
}

#[async_trait]
impl Message for ArteryHeartbeatRsp {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        actor.receive_heartbeat_rsp(context, self.uid)
    }
}