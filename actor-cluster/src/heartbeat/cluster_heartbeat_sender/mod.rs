use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use tracing::trace;

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster::Cluster;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_sender_cluster_event::HeartbeatSenderClusterEvent;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_tick::HeartbeatTick;
use crate::member::Member;
use crate::unique_address::UniqueAddress;

mod heartbeat_sender_cluster_event;
mod heartbeat_tick;
pub(crate) mod heartbeat_rsp;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatSender {
    self_member: Option<Member>,
    active_receivers: HashSet<UniqueAddress>,
    event_adapter: ActorRef,
    key: Option<ScheduleKey>,
}

#[async_trait]
impl Actor for ClusterHeartbeatSender {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Cluster::get(context.system()).subscribe_cluster_event(self.event_adapter.clone());
        let myself = context.myself().clone();
        let key = context.system().scheduler().schedule_with_fixed_delay(None, Duration::from_secs(5), move || {
            myself.cast_ns(HeartbeatTick);
        });
        self.key = Some(key);
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} stopped", context.myself());
        if let Some(key) = self.key.take() {
            key.cancel();
        }
        Cluster::get(context.system()).unsubscribe_cluster_event(&self.event_adapter);
        Ok(())
    }
}

impl ClusterHeartbeatSender {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let event_adapter = context.message_adapter(|m| DynMessage::user(HeartbeatSenderClusterEvent(m)));
        Self {
            active_receivers: Default::default(),
            event_adapter,
            self_member: None,
            key: None,
        }
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_sender"
    }
}