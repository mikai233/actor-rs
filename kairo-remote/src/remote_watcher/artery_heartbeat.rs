use async_trait::async_trait;
use bincode::{Decode, Encode};

use kairo_core::CMessageCodec;
use kairo_core::Message;
use kairo_core::actor::context::ActorContext;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct ArteryHeartbeat;

#[async_trait]
impl Message for ArteryHeartbeat {
    type A = RemoteWatcher;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        actor.receive_heartbeat(context)
    }
}
