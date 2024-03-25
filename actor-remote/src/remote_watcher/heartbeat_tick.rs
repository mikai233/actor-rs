use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::remote_watcher::artery_heartbeat::ArteryHeartbeat;
use crate::remote_watcher::expected_first_heartbeat::ExpectedFirstHeartbeat;
use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(super) struct HeartbeatTick;

#[async_trait]
impl Message for HeartbeatTick {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let watching_nodes = actor.watchee_by_nodes.keys();
        for addr in watching_nodes {
            if actor.unreachable.contains(addr).not() {
                debug!("Sending Heartbeat to [{}]", addr);
            } else {
                debug!("Sending first Heartbeat to [{}]", addr);
                let myself = context.myself().clone();
                let msg = ExpectedFirstHeartbeat {
                    from: addr.clone(),
                };
                let elements = myself.path().elements();
                let elements_str = elements.iter().map(|e| e.as_str());
                let path = RootActorPath::new(addr.clone(), "/").descendant(elements_str);
                let selection = context.actor_selection(ActorSelectionPath::FullPath(path))?;
                selection.tell(ArteryHeartbeat.into_dyn(), Some(context.myself().clone()));
                context.system().scheduler.schedule_once(actor.heartbeat_expected_response_after, move || {
                    myself.cast_ns(msg);
                });
            }
        }
        Ok(())
    }
}