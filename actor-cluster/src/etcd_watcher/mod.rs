use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{Watcher, WatchOptions, WatchStream};
use futures::task::ArcWake;
use tracing::{debug, warn};

use actor_core::Actor;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::etcd_client::EtcdClient;

use crate::etcd_watcher::poll_message::PollMessage;
use crate::etcd_watcher::retry_watch::RetryWatch;

mod poll_message;
pub mod watch_resp;
mod cancel_watch;
mod retry_watch;

pub struct EtcdWatcher {
    client: EtcdClient,
    key: String,
    options: Option<WatchOptions>,
    watcher: Option<Watcher>,
    watcher_stream: Option<WatchStream>,
    waker: futures::task::Waker,
    watch_receiver: ActorRef,
}

#[async_trait]
impl Actor for EtcdWatcher {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.watch(context).await;
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.cancel().await;
        }
        Ok(())
    }
}

impl EtcdWatcher {
    pub fn new(myself: ActorRef, client: EtcdClient, key: String, options: Option<WatchOptions>, receiver: ActorRef) -> Self {
        let waker = futures::task::waker(Arc::new(WatchWaker { watcher: myself }));
        Self {
            client,
            key,
            options,
            watcher: None,
            watcher_stream: None,
            waker,
            watch_receiver: receiver,
        }
    }

    async fn watch(&mut self, context: &mut ActorContext) {
        debug!("{} start watch {}", context.myself() ,self.key);
        match self.client.watch(self.key.as_bytes(), self.options.clone()).await {
            Ok((watcher, watch_stream)) => {
                self.watcher = Some(watcher);
                self.watcher_stream = Some(watch_stream);
                context.myself().cast_ns(PollMessage);
            }
            Err(error) => {
                let retry = Duration::from_secs(3);
                warn!("{} watch {} {:?}, retry after {:?}", context.myself(), self.key, error, retry);
                let myself = context.myself().clone();
                context.system().scheduler().schedule_once(retry, move || { myself.cast_ns(RetryWatch); });
            }
        };
    }
}

struct WatchWaker {
    watcher: ActorRef,
}

impl ArcWake for WatchWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.watcher.cast_ns(PollMessage);
    }
}
