use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::warn;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(super) struct ReapUnreachableTick;

#[async_trait]
impl Message for ReapUnreachableTick {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let watching_nodes = actor.watchee_by_nodes.keys();
        for addr in watching_nodes {
            if actor.unreachable.contains(addr).not() && actor.failure_detector.is_available(addr).not() {
                warn!("Detected unreachable: [{}]", addr);
                //TODO quarantine
                actor.publish_address_terminated(addr.clone());
                actor.unreachable.insert(addr.clone());
            }
        }
        Ok(())
    }
}