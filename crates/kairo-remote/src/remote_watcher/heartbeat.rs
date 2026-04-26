use async_trait::async_trait;
use bincode::{Decode, Encode};

use kairo_core::actor::context::ActorContext;
use kairo_core::{Message, MessageCodec};

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct Heartbeat;

#[async_trait]
impl Message for Heartbeat {
    type A = RemoteWatcher;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        actor.receive_heartbeat(context)
    }
}
