use actor_core::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
    message::handler::MessageHandler,
    Message, MessageCodec,
};
use serde::{Deserialize, Serialize};

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Clone, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[cloneable]
#[display("ArteryHeartbeat")]
pub(crate) struct ArteryHeartbeat;

impl MessageHandler<RemoteWatcher> for ArteryHeartbeat {
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
