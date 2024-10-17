use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};
use serde::{Deserialize, Serialize};

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[display("Heartbeat")]
pub(crate) struct Heartbeat;

impl MessageHandler<RemoteWatcher> for Heartbeat {
    fn handle(
        actor: &mut RemoteWatcher,
        ctx: &mut <RemoteWatcher as Actor>::Context,
        _: Self,
        sender: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        actor.receive_heartbeat(ctx, sender);
        Ok(Behavior::same())
    }
}
