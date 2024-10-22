use std::time::Duration;

use actor_core::actor::Actor;
use ahash::HashSet;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;

use crate::cluster::Cluster;
use crate::heartbeat::cluster_heartbeat_sender::cluster_event::ClusterEventWrap;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_tick::HeartbeatTick;
use crate::member::Member;
use crate::unique_address::UniqueAddress;

mod cluster_event;
pub(crate) mod heartbeat_rsp;
mod heartbeat_tick;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatSender {
    self_member: Option<Member>,
    active_receivers: HashSet<UniqueAddress>,
    key: Option<ScheduleKey>,
}

impl Actor for ClusterHeartbeatSender {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        trace!("{} started", ctx.myself());
        Cluster::get(ctx.system()).subscribe(ctx.myself().clone(), |event| {
            ClusterEventWrap(event).into_dyn()
        })?;
        let myself = ctx.myself().clone();
        let key = ctx.system().scheduler.schedule_with_fixed_delay(
            None,
            Duration::from_secs(5),
            move || {
                myself.cast_ns(HeartbeatTick);
            },
        );
        self.key = Some(key);
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        trace!("{} stopped", ctx.myself());
        if let Some(key) = self.key.take() {
            key.cancel();
        }
        Cluster::get(ctx.system()).unsubscribe_cluster_event(ctx.myself())?;
        Ok(())
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

impl ClusterHeartbeatSender {
    pub(crate) fn new() -> Self {
        Self {
            active_receivers: Default::default(),
            self_member: None,
            key: None,
        }
    }

    pub(crate) fn props() -> Props {
        Props::new(|| Ok(Self::new()))
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_sender"
    }
}
