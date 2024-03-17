use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::ActorRef;
use actor_core::ext::etcd_client::EtcdClient;

use crate::etcd_actor::keep_alive::KeepAliveFailed;
use crate::etcd_actor::lease::Lease;
use crate::etcd_actor::poll_keep_alive_resp::PollKeepAliveRespWaker;
use crate::etcd_actor::poll_watch_resp::PollWatchRespWaker;
use crate::etcd_actor::watcher::Watcher;

mod lease;
mod poll_keep_alive_resp;
pub mod cancel_keep_alive;
pub mod watch;
mod keeper;
mod keeper_keep_alive_failed;
pub mod keep_alive;
mod unwatch;
mod watch_started;
mod watcher;
mod poll_watch_resp;

#[derive(Debug)]
pub struct EtcdActor {
    client: EtcdClient,
    keep_alive_resp_waker: futures::task::Waker,
    watch_resp_waker: futures::task::Waker,
    lease: HashMap<i64, Lease>,
    watcher: HashMap<i64, Watcher>,
}

impl EtcdActor {
    pub fn new(context: &mut ActorContext, client: EtcdClient) -> Self {
        let keep_alive_resp_waker = futures::task::waker(Arc::new(PollKeepAliveRespWaker { actor: context.myself().clone() }));
        let watch_resp_waker = futures::task::waker(Arc::new(PollWatchRespWaker { actor: context.myself().clone() }));
        Self {
            client,
            keep_alive_resp_waker,
            watch_resp_waker,
            lease: Default::default(),
            watcher: Default::default(),
        }
    }

    fn keep_alive_failed(id: i64, applicant: &ActorRef, error: Option<etcd_client::Error>) {
        let keep_alive_failed = KeepAliveFailed {
            id,
            error,
        };
        applicant.tell(
            DynMessage::orphan(keep_alive_failed),
            ActorRef::no_sender(),
        );
    }
}

#[async_trait]
impl Actor for EtcdActor {}