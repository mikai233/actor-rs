use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use tracing::debug;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::timers::Timers;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::ext::option_ext::OptionExt;

use crate::shard_coordinator::rebalance_done::RebalanceDone;
use crate::shard_coordinator::rebalance_worker::receive_timeout::ReceiveTimeout;
use crate::shard_region::begin_handoff::BeginHandoff;
use crate::shard_region::ImShardId;

mod receive_timeout;

#[derive(Debug)]
pub(super) struct RebalanceWorker {
    type_name: String,
    shard: ImShardId,
    shard_region_from: ActorRef,
    handoff_timeout: Duration,
    regions: HashSet<ActorRef>,
    is_rebalance: bool,
    remaining: HashSet<ActorRef>,
    timers: Timers,
}

#[async_trait]
impl Actor for RebalanceWorker {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        for region in &self.regions {
            region.cast(BeginHandoff { shard: self.shard.clone().into() }, Some(context.myself().clone()));
        }
        if self.is_rebalance {
            debug!("{}: Rebalance [{}] from [{}] regions", self.type_name, self.shard, self.regions.len());
        } else {
            debug!(
                "{}: Shutting down shard [{}] from region [{}]. Asking [{}] region(s) to hand-off shard",
                self.type_name,
                self.shard,
                self.shard_region_from,
                self.regions.len(),
            );
        }
        self.timers.start_single_timer(self.handoff_timeout, ReceiveTimeout.into_dyn(), context.myself().clone());
        Ok(())
    }
}

impl RebalanceWorker {
    pub(super) fn new(
        context: &mut ActorContext,
        type_name: String,
        shard: ImShardId,
        shard_region_from: ActorRef,
        handoff_timeout: Duration,
        regions: HashSet<ActorRef>,
        is_rebalance: bool,
    ) -> anyhow::Result<Self> {
        let timers = Timers::new(context)?;
        let remaining = regions.clone();
        let myself = Self {
            type_name,
            shard,
            shard_region_from,
            handoff_timeout,
            regions,
            is_rebalance,
            remaining,
            timers,
        };
        Ok(myself)
    }

    fn done(&self, context: &mut ActorContext, ok: bool) {
        context.parent().foreach(|parent| {
            parent.cast(RebalanceDone { shard: self.shard.clone(), ok }, Some(context.myself().clone()));
        });
        context.stop(context.myself());
    }
}