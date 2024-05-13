use std::ops::Deref;
use std::sync::Arc;
use std::task::Poll;

use async_trait::async_trait;
use futures::StreamExt;
use futures::task::ArcWake;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::watch::WatchResp;

#[derive(Debug, EmptyCodec)]
pub(super) struct PollWatchResp;

#[async_trait]
impl Message for PollWatchResp {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut cx = futures::task::Context::from_waker(&actor.watch_resp_waker);
        let mut failed = vec![];
        for (id, watcher) in &mut actor.watcher {
            while let Poll::Ready(resp) = watcher.stream.poll_next_unpin(&mut cx) {
                match resp {
                    None => {
                        watcher.applicant.cast_orphan_ns(WatchResp::Failed(None));
                        failed.push(*id);
                        break;
                    }
                    Some(resp) => {
                        match resp {
                            Ok(resp) => {
                                watcher.applicant.cast_orphan_ns(WatchResp::Update(resp));
                            }
                            Err(error) => {
                                watcher.applicant.cast_orphan_ns(WatchResp::Failed(Some(error)));
                                failed.push(*id);
                                break;
                            }
                        }
                    }
                }
            }
        }
        for id in failed {
            actor.watcher.remove(&id);
        }
        Ok(())
    }
}

pub(super) struct PollWatchRespWaker {
    pub(super) actor: ActorRef,
}

impl Deref for PollWatchRespWaker {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.actor
    }
}

impl ArcWake for PollWatchRespWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.cast_ns(PollWatchResp);
    }
}