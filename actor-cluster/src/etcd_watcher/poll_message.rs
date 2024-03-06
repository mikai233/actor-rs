use std::task::Poll;

use async_trait::async_trait;
use futures::StreamExt;
use tracing::warn;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_derive::EmptyCodec;

use crate::etcd_watcher::EtcdWatcher;
use crate::etcd_watcher::watch_resp::WatchResp;

#[derive(Debug, EmptyCodec)]
pub(super) struct PollMessage;

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