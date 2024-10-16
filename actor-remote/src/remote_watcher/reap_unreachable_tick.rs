use std::ops::Not;

use actor_core::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
    message::handler::MessageHandler,
    Message,
};
use tracing::warn;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Message, derive_more::Display)]
#[display("ReapUnreachableTick")]
pub(super) struct ReapUnreachableTick;

impl MessageHandler<RemoteWatcher> for ReapUnreachableTick {
    fn handle(
        actor: &mut RemoteWatcher,
        ctx: &mut <RemoteWatcher as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        let watching_nodes = actor.watchee_by_nodes.keys();
        for addr in watching_nodes {
            if actor.unreachable.contains(addr).not()
                && actor.failure_detector.is_available(addr).not()
            {
                warn!("Detected unreachable: [{}]", addr);
                //TODO quarantine
                actor.publish_address_terminated(addr.clone());
                actor.unreachable.insert(addr.clone());
            }
        }
        Ok(Behavior::same())
    }
}
