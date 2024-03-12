use std::ops::Deref;
use std::sync::Arc;
use std::task::Poll;

use async_trait::async_trait;
use futures::StreamExt;
use futures::task::ArcWake;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_derive::EmptyCodec;

use crate::etcd_actor::etcd_cmd_resp::EtcdCmdResp;
use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::keep_alive::KeepAliveFailed;

#[derive(Debug, EmptyCodec)]
pub(super) struct PollKeepAliveResp;

#[async_trait]
impl Message for PollKeepAliveResp {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut cx = futures::task::Context::from_waker(&actor.keep_alive_resp_waker);
        let mut failed = vec![];
        for (id, lease) in &mut actor.lease {
            while let Poll::Ready(resp) = lease.stream.poll_next_unpin(&mut cx) {
                match resp {
                    None => {
                        let keep_alive_failed = KeepAliveFailed {
                            id: *id,
                            error: None,
                        };
                        lease.watcher.tell(
                            DynMessage::orphan(EtcdCmdResp::KeepAliveFailed(keep_alive_failed)),
                            Some(context.myself().clone()),
                        );
                        failed.push(*id);
                    }
                    Some(resp) => {
                        if let Some(error) = resp.err() {
                            let keep_alive_failed = KeepAliveFailed {
                                id: *id,
                                error: Some(error),
                            };
                            lease.watcher.tell(
                                DynMessage::orphan(EtcdCmdResp::KeepAliveFailed(keep_alive_failed)),
                                Some(context.myself().clone()),
                            );
                            failed.push(*id);
                        }
                    }
                }
            }
        }
        for id in failed {
            actor.lease.remove(&id);
        }
        Ok(())
    }
}

pub(super) struct PollKeepAliveRespWaker {
    pub(super) actor: ActorRef,
}

impl Deref for PollKeepAliveRespWaker {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.actor
    }
}

impl ArcWake for PollKeepAliveRespWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.cast_ns(PollKeepAliveResp);
    }
}