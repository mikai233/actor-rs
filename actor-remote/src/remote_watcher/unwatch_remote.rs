use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, EmptyCodec)]
pub(crate) struct UnwatchRemote {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

#[async_trait]
impl Message for UnwatchRemote {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let Self { watchee, watcher } = *self;
        actor.remove_watch(context, watchee, watcher);
        Ok(())
    }
}