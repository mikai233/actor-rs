use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, EmptyCodec)]
pub(crate) struct WatchRemote {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

#[async_trait]
impl Message for WatchRemote {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { watchee, watcher } = *self;
        actor.add_watch(context, watchee, watcher)
    }
}