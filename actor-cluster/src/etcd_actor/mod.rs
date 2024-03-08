use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use actor_core::Actor;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::ext::etcd_client::EtcdClient;

use crate::etcd_actor::lease::Lease;

mod lease;
mod poll_keep_alive_resp;
pub mod keep_alive_failed;
pub mod cancel_keep_alive;
pub mod put;
pub mod delete;
pub mod watch;
mod keeper;
mod keeper_keep_alive_failed;
pub mod keep_alive;

#[derive(Debug)]
pub struct EtcdActor {
    client: EtcdClient,
    keep_alive_resp_waker: futures::task::Waker,
    lease: HashMap<i64, Lease>,
    interval: Duration,
    tick: Option<ScheduleKey>,
}

#[async_trait]
impl Actor for EtcdActor {}