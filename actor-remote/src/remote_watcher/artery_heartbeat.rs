use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct ArteryHeartbeat;

#[async_trait]
impl Message for ArteryHeartbeat {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.receive_heartbeat(context)
    }
}