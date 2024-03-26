use async_trait::async_trait;
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, EmptyCodec)]
pub(super) struct WatcheeTerminated(pub(super) Terminated);

impl WatcheeTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for WatcheeTerminated {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let watchee = self.0.actor;
        debug!("Watchee terminated: [{}]", watchee.path());
        todo!()
    }
}