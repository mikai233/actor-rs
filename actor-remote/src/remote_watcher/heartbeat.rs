use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::{Message, MessageCodec};
use actor_core::actor::context::ActorContext;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct Heartbeat;

#[async_trait]
impl Message for Heartbeat {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.receive_heartbeat(context)
    }
}