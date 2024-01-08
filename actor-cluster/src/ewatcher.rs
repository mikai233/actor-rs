use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use async_recursion::async_recursion;
use async_trait::async_trait;
use etcd_client::{Client, Watcher, WatchOptions, WatchResponse, WatchStream};
use futures::StreamExt;
use futures::task::ArcWake;
use tracing::warn;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::context::{ActorContext, Context};
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

pub struct EWatcher {
    eclient: Client,
    key: String,
    options: Option<WatchOptions>,
    watcher: Option<Watcher>,
    watcher_stream: Option<WatchStream>,
    waker: futures::task::Waker,
    watch_receiver: ActorRef,
}

impl Debug for EWatcher {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EWatcher")
            .field("eclient", &"..")
            .field("key", &self.key)
            .field("options", &self.options)
            .field("watcher", &self.watcher)
            .field("watcher_stream", &self.watcher_stream)
            .field("waker", &self.waker)
            .field("watch_receiver", &self.watch_receiver)
            .finish()
    }
}

#[async_trait]
impl Actor for EWatcher {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.watch(context).await;
        Ok(())
    }
}

impl EWatcher {
    #[async_recursion]
    async fn watch(&mut self, context: &mut ActorContext) {
        match self.eclient.watch(self.key.as_bytes(), self.options.clone()).await {
            Ok((watcher, watch_stream)) => {
                self.watcher = Some(watcher);
                self.watcher_stream = Some(watch_stream);
                context.myself().cast_ns(PollMessage);
            }
            Err(error) => {
                warn!("watch {} {:?}, sleep 3s and try rewatch it", self.key, error);
                tokio::time::sleep(Duration::from_secs(3)).await;
                self.watch(context).await;
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
    type A = EWatcher;

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
                    warn!("watch {} stream closed, try rewatch it", actor.key);
                    actor.watch(context).await;
                }
                Some(Ok(resp)) => {
                    actor.watch_receiver.tell(DynMessage::orphan(WatchRespWrap(resp)), ActorRef::no_sender());
                }
                Some(Err(error)) => {
                    warn!("watch {} {:?}, try rewatch it", actor.key, error);
                    actor.watch(context).await;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub struct WatchRespWrap(WatchResponse);

impl Deref for WatchRespWrap {
    type Target = WatchResponse;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, EmptyCodec)]
pub struct CancelWatch;

#[async_trait]
impl Message for CancelWatch {
    type A = EWatcher;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(watcher) = &mut actor.watcher {
            watcher.cancel().await?;
        }
        Ok(())
    }
}