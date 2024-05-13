use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::poll_watch_resp::PollWatchResp;
use crate::etcd_actor::watcher::Watcher;

#[derive(Debug, EmptyCodec)]
pub(super) struct WatchStarted {
    pub(super) watcher: etcd_client::Watcher,
    pub(super) stream: etcd_client::WatchStream,
    pub(super) applicant: ActorRef,
}

#[async_trait]
impl Message for WatchStarted {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let id = self.watcher.watch_id();
        debug!("watch started with watch id {} and applicant {}", id, self.applicant);
        let watcher = Watcher {
            watcher: self.watcher,
            stream: self.stream,
            applicant: self.applicant,
        };
        actor.watcher.insert(id, watcher);
        context.myself().cast_ns(PollWatchResp);
        Ok(())
    }
}