use std::collections::hash_map::Entry;
use std::ops::Not;
use std::time::Duration;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::death_watch_notification::DeathWatchNotification;
use actor_core::message::terminated::Terminated;
use ahash::{HashMap, HashSet, HashSetExt};
use anyhow::anyhow;
use artery_heartbeat::ArteryHeartbeat;
use expected_first_heartbeat::ExpectedFirstHeartbeat;
use heartbeat::Heartbeat;
use heartbeat_rsp::HeartbeatRsp;
use tracing::debug;

use actor_core::actor::address::Address;
use actor_core::actor::context::Context;
use actor_core::actor::props::Props;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::event::address_terminated_topic::AddressTerminatedTopic;
use actor_core::message::address_terminated::AddressTerminated;
use actor_core::message::watch::Watch;
use unwatch_remote::UnwatchRemote;
use watch_remote::WatchRemote;

use crate::failure_detector::failure_detector_registry::FailureDetectorRegistry;
use crate::remote_watcher::artery_heartbeat_rsp::ArteryHeartbeatRsp;
use crate::remote_watcher::heartbeat_tick::HeartbeatTick;
use crate::remote_watcher::reap_unreachable_tick::ReapUnreachableTick;

pub(crate) mod artery_heartbeat;
pub(crate) mod artery_heartbeat_rsp;
mod expected_first_heartbeat;
pub(crate) mod heartbeat;
pub(crate) mod heartbeat_rsp;
mod heartbeat_tick;
mod reap_unreachable_tick;
pub(crate) mod unwatch_remote;
pub(crate) mod watch_remote;

#[derive(Debug)]
pub struct RemoteWatcher {
    failure_detector: Box<dyn FailureDetectorRegistry<A = Address>>,
    heartbeat_interval: Duration,
    unreachable_reaper_interval: Duration,
    heartbeat_expected_response_after: Duration,
    watching: HashMap<ActorRef, HashSet<ActorRef>>,
    watchee_by_nodes: HashMap<Address, HashSet<ActorRef>>,
    unreachable: HashSet<Address>,
    address_uids: HashMap<Address, i64>,
    heartbeat_task: Option<ScheduleKey>,
    failure_detector_reaper_task: Option<ScheduleKey>,
    address_terminated_topic: AddressTerminatedTopic,
}

impl Actor for RemoteWatcher {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let myself = ctx.myself().clone();
        let heartbeat_task = ctx.system().scheduler.schedule_with_fixed_delay(
            Some(self.heartbeat_interval),
            self.heartbeat_interval,
            move || {
                myself.cast_ns(HeartbeatTick);
            },
        );
        self.heartbeat_task = Some(heartbeat_task);
        let myself = ctx.myself().clone();
        let failure_detector_reaper_task = ctx.system().scheduler.schedule_with_fixed_delay(
            Some(self.unreachable_reaper_interval),
            self.unreachable_reaper_interval,
            move || {
                myself.cast_ns(ReapUnreachableTick);
            },
        );
        self.failure_detector_reaper_task = Some(failure_detector_reaper_task);
        Ok(())
    }

    fn stopped(&mut self, _: &mut Self::Context) -> anyhow::Result<()> {
        if let Some(task) = self.heartbeat_task.take() {
            task.cancel();
        }
        if let Some(task) = self.failure_detector_reaper_task.take() {
            task.cancel();
        }
        Ok(())
    }

    fn receive(&self) -> Receive<Self> {
        Receive::new()
            .handle::<ArteryHeartbeatRsp>()
            .handle::<ArteryHeartbeat>()
            .handle::<ExpectedFirstHeartbeat>()
            .handle::<HeartbeatRsp>()
            .handle::<HeartbeatTick>()
            .handle::<Heartbeat>()
            .handle::<ReapUnreachableTick>()
            .handle::<UnwatchRemote>()
            .handle::<WatchRemote>()
            .is::<Terminated>(|actor, _, msg, _, _| {
                actor.terminated(msg.actor, msg.existence_confirmed, msg.address_terminated);
                Ok(Behavior::same())
            })
    }
}

impl RemoteWatcher {
    pub fn props<F>(registry: F) -> Props
    where
        F: FailureDetectorRegistry<A = Address> + 'static,
    {
        Props::new_with_ctx(move |ctx| Ok(Self::new(ctx, registry)))
    }

    pub fn new<F>(context: &mut Context, registry: F) -> Self
    where
        F: FailureDetectorRegistry<A = Address> + 'static,
    {
        Self {
            failure_detector: Box::new(registry),
            heartbeat_interval: Duration::from_secs(1),
            unreachable_reaper_interval: Duration::from_secs(1),
            heartbeat_expected_response_after: Duration::from_secs(1),
            watching: Default::default(),
            watchee_by_nodes: Default::default(),
            unreachable: Default::default(),
            address_uids: Default::default(),
            heartbeat_task: None,
            failure_detector_reaper_task: None,
            address_terminated_topic: AddressTerminatedTopic::get(context.system()).clone(),
        }
    }

    pub fn add_watch(
        &mut self,
        ctx: &mut <Self as Actor>::Context,
        watchee: ActorRef,
        watcher: ActorRef,
    ) -> anyhow::Result<()> {
        debug_assert_ne!(&watcher, ctx.myself());
        debug!("Watching: [{} -> {}]", watcher, watchee);
        match self.watching.entry(watchee.clone()) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(watcher);
            }
            Entry::Vacant(v) => {
                let mut watchers = HashSet::new();
                watchers.insert(watcher);
                v.insert(watchers);
            }
        }
        self.watch_node(watchee.clone());
        //可能是Watcher不同但是Watchee是相同的，这种情况会watch多次
        if ctx.is_watching(&watchee).not() {
            ctx.watch(&watchee);
        }
        Ok(())
    }

    pub fn remove_watch(&mut self, context: &mut Context, watchee: ActorRef, watcher: ActorRef) {
        debug_assert_ne!(&watcher, context.myself());
        if let Some(watchers) = self.watching.get_mut(&watchee) {
            watchers.remove(&watcher);
            if watchers.is_empty() {
                debug!("Unwatching: [{} -> {}]", watcher, watchee);
                debug!("Cleanup self watch of [{}]", watchee.path());
                context.unwatch(&watchee);
                self.remove_watchee(&watchee);
            }
        }
    }

    pub fn terminated(
        &mut self,
        watchee: ActorRef,
        existence_confirmed: bool,
        address_terminated: bool,
    ) {
        debug!("Watchee terminated: [{}]", watchee.path());
        // When watchee is stopped it sends DeathWatchNotification to this RemoteWatcher,
        // which will propagate it to all watchers of this watchee.
        // address_terminated case is already handled by the watcher itself in DeathWatch trait
        if !address_terminated {
            if let Some(watchers) = self.watching.get(&watchee) {
                let notify = DeathWatchNotification {
                    actor: watchee.clone(),
                    existence_confirmed,
                    address_terminated,
                };
                for watcher in watchers {
                    watcher.cast_ns(notify.clone());
                }
            }
        }
        self.remove_watchee(&watchee);
    }

    pub fn remove_watchee(&mut self, watchee: &ActorRef) {
        let watchee_address = watchee.path().address();
        self.watching.remove(&watchee);
        if let Some(watchees) = self.watchee_by_nodes.get_mut(watchee_address) {
            watchees.remove(&watchee);
            if watchees.is_empty() {
                debug!("Unwatched last watchee of node: [{}]", watchee_address);
                self.unwatch_node(watchee_address);
            }
        }
    }

    pub fn watch_node(&mut self, watchee: ActorRef) {
        let watchee_address = watchee.path().address();
        if self.watchee_by_nodes.contains_key(watchee_address).not()
            && self.unreachable.contains(watchee_address)
        {
            self.unreachable.remove(watchee_address);
            self.failure_detector.remove(watchee_address);
        }
        match self.watchee_by_nodes.entry(watchee_address.clone()) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(watchee);
            }
            Entry::Vacant(v) => {
                let mut watchess = HashSet::new();
                watchess.insert(watchee);
                v.insert(watchess);
            }
        }
    }

    pub fn unwatch_node(&mut self, watche_address: &Address) {
        self.watchee_by_nodes.remove(watche_address);
        self.address_uids.remove(watche_address);
        self.failure_detector.remove(watche_address);
    }

    pub fn receive_heartbeat_rsp(
        &mut self,
        context: &mut <Self as Actor>::Context,
        uid: i64,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<()> {
        let sender = sender.ok_or(anyhow!("receive_heartbeat_rsp sender is none"))?;
        let from = sender.path().address();
        if self.failure_detector.is_monitoring(from) {
            debug!("Received heartbeat rsp from [{}]", from);
        } else {
            debug!("Received first heartbeat rsp from [{}]", from);
        }
        if self.watchee_by_nodes.contains_key(from) && self.unreachable.contains(from).not() {
            if self
                .address_uids
                .get(from)
                .map(|x| x != &uid)
                .unwrap_or(true)
            {
                self.re_watch(context.myself().clone(), &from);
            }
            self.address_uids.insert(from.clone(), uid);
            self.failure_detector.heartbeat(from.clone());
        }
        Ok(())
    }

    pub fn re_watch(&self, watcher: ActorRef, address: &Address) {
        if let Some(watchees) = self.watchee_by_nodes.get(address) {
            for watchee in watchees {
                debug!("Re-watch [{} -> {}]", watcher.path(), watchee.path());
                let watch = Watch {
                    watchee: watchee.clone(),
                    watcher: watcher.clone(),
                };
                watchee.cast_ns(watch);
            }
        }
    }

    pub fn receive_heartbeat(&self, context: &mut Context, sender: Option<ActorRef>) {
        if let Some(sender) = sender {
            sender.cast_ns(ArteryHeartbeatRsp::new(context.system().uid));
        }
    }

    pub fn publish_address_terminated(&self, address: Address) {
        debug!("Publish AddressTerminated [{}]", address);
        self.address_terminated_topic
            .publish(AddressTerminated { address });
    }
}
