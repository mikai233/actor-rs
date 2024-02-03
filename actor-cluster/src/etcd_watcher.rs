use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{Watcher, WatchOptions, WatchResponse, WatchStream};
use futures::StreamExt;
use futures::task::ArcWake;
use tracing::{debug, warn};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::etcd_client::EtcdClient;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

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
                warn!("{} watch {} {:?}, sleep 3s and try rewatch it", context.myself(), self.key, error);
                let myself = context.myself().clone();
                context.system().scheduler().schedule_once(Duration::from_secs(3), move || {
                    myself.cast_ns(RetryWatch);
                });
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


#[derive(Debug, EmptyCodec)]
struct PollMessage;

#[async_trait]
impl Message for PollMessage {
    type A = EtcdWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let stream = actor.watcher_stream.as_mut().unwrap();
        let poll = {
            let mut ctx = futures::task::Context::from_waker(&actor.waker);
            stream.poll_next_unpin(&mut ctx)
        };
        if let Poll::Ready(watch_response) = poll {
            match watch_response {
                None => {
                    //stream closed, rewatch
                    warn!("{} watch {} stream closed, try rewatch it", context.myself(), actor.key);
                    actor.watch(context).await;
                }
                Some(Ok(resp)) => {
                    actor.watch_receiver.tell(
                        DynMessage::orphan(WatchResp { key: actor.key.clone(), resp }),
                        ActorRef::no_sender(),
                    );
                }
                Some(Err(error)) => {
                    warn!("{} watch {} {:?}, try rewatch it", context.myself(), actor.key, error);
                    actor.watch(context).await;
                }
            }
            context.myself().cast_ns(PollMessage);
        }
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub struct WatchResp {
    pub key: String,
    pub resp: WatchResponse,
}

impl Deref for WatchResp {
    type Target = WatchResponse;

    fn deref(&self) -> &Self::Target {
        &self.resp
    }
}

/// cancel key watcher and stop watch actor
#[derive(Debug, EmptyCodec)]
pub struct CancelWatch;

#[async_trait]
impl Message for CancelWatch {
    type A = EtcdWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(watcher) = &mut actor.watcher {
            watcher.cancel().await?;
            context.stop(context.myself());
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct RetryWatch;

#[async_trait]
impl Message for RetryWatch {
    type A = EtcdWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.watch(context).await;
        Ok(())
    }
}