use std::ops::Not;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::debug;

use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};

use crate::remote_watcher::artery_heartbeat::ArteryHeartbeat;
use crate::remote_watcher::expected_first_heartbeat::ExpectedFirstHeartbeat;
use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Message, derive_more::Display)]
#[display("HeartbeatTick")]
pub(super) struct HeartbeatTick;

impl MessageHandler<RemoteWatcher> for HeartbeatTick {
    fn handle(
        actor: &mut RemoteWatcher,
        ctx: &mut <RemoteWatcher as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        let watching_nodes = actor.watchee_by_nodes.keys();
        for addr in watching_nodes {
            if actor.unreachable.contains(addr).not() {
                debug!("Sending Heartbeat to [{}]", addr);
            } else {
                debug!("Sending first Heartbeat to [{}]", addr);
                let myself = ctx.myself().clone();
                let msg = ExpectedFirstHeartbeat { from: addr.clone() };
                ctx.system().scheduler.schedule_once(
                    actor.heartbeat_expected_response_after,
                    move || {
                        myself.cast_ns(msg);
                    },
                );
            }
            let elements = ctx.myself().path().elements();
            let elements_str = elements.iter().map(|e| e.as_str());
            let path = RootActorPath::new(addr.clone(), "/").descendant(elements_str);
            let selection = ctx.actor_selection(ActorSelectionPath::FullPath(path))?;
            selection.tell(Box::new(ArteryHeartbeat), Some(ctx.myself().clone()));
        }
        Ok(Behavior::same())
    }
}
