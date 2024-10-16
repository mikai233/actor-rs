use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Message, derive_more::Display)]
#[display("UnwatchRemote {{ watchee: {}, watcher: {} }}", watchee, watcher)]
pub(crate) struct UnwatchRemote {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

impl MessageHandler<RemoteWatcher> for UnwatchRemote {
    fn handle(
        actor: &mut RemoteWatcher,
        ctx: &mut <RemoteWatcher as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        let Self { watchee, watcher } = message;
        actor.remove_watch(ctx, watchee, watcher);
        Ok(Behavior::same())
    }
}
