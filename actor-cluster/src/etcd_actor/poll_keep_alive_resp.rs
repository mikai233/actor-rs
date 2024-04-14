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

#[derive(Debug, EmptyCodec)]
pub(super) struct PollKeepAliveResp;

#[async_trait]
impl Message for PollKeepAliveResp {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let mut cx = futures::task::Context::from_waker(&actor.keep_alive_resp_waker);
        let mut failed = vec![];
        for (id, lease) in &mut actor.lease {
            while let Poll::Ready(resp) = lease.stream.poll_next_unpin(&mut cx) {
                match resp {
                    None => {
                        EtcdActor::keep_alive_failed(*id, &lease.applicant, None);
                        failed.push(*id);
                    }
                    Some(resp) => {
                        if let Some(error) = resp.err() {
                            EtcdActor::keep_alive_failed(*id, &lease.applicant, Some(error));
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