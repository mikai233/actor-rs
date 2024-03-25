use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::time::Duration;

use async_trait::async_trait;
use tracing::debug;

use actor_core::Actor;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefSystemExt};
use actor_core::message::death_watch_notification::DeathWatchNotification;
use actor_core::message::watch::Watch;

use crate::failure_detector::failure_detector_registry::FailureDetectorRegistry;
use crate::remote_watcher::watchee_terminated::WatcheeTerminated;

mod heartbeat_tick;
mod heartbeat;
mod heartbeat_rsp;
mod watch_remote;
mod unwatch_remote;
mod watchee_terminated;
mod artery_heartbeat;
mod artery_heartbeat_rsp;
mod reap_unreachable_tick;
mod expected_first_heartbeat;

#[derive(Debug)]
pub struct RemoteWatcher {
    failure_detector: Box<dyn FailureDetectorRegistry<A=Address>>,
    heartbeat_interval: Duration,
    unreachable_reaper_interval: Duration,
    heartbeat_expected_response_after: Duration,
    watching: HashMap<ActorRef, HashSet<ActorRef>>,
    watchee_by_nodes: HashMap<Address, HashSet<ActorRef>>,
    unreachable: HashSet<Address>,
    address_uids: HashMap<Address, i64>,
    heartbeat_task: Option<ScheduleKey>,
    failure_detector_reaper_task: Option<ScheduleKey>,
}

#[async_trait]
impl Actor for RemoteWatcher {
    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        if let Some(task) = self.heartbeat_task.take() {
            task.cancel();
        }
        if let Some(task) = self.failure_detector_reaper_task.take() {
            task.cancel();
        }
        Ok(())
    }
}

impl RemoteWatcher {
    fn add_watch(&mut self, context: &mut ActorContext, watchee: ActorRef, watcher: ActorRef) {
        debug_assert_ne!(&watcher, context.myself());
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
        context.watch(WatcheeTerminated(watchee));
    }

    fn remove_watch(&mut self, context: &mut ActorContext, watchee: ActorRef, watcher: ActorRef) {
        debug_assert_ne!(&watcher, context.myself());
    }

    fn remove_watchee(&mut self, watchee: &ActorRef) {
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

    fn watch_node(&mut self, watchee: ActorRef) {
        let watchee_address = watchee.path().address();
        if self.watchee_by_nodes.contains_key(watchee_address).not() && self.unreachable.contains(watchee_address) {
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

    fn unwatch_node(&mut self, watche_address: &Address) {
        self.watchee_by_nodes.remove(watche_address);
        self.address_uids.remove(watche_address);
        self.failure_detector.remove(watche_address);
    }

    fn terminated(&mut self, watchee: &ActorRef, existence_confirmed: bool, address_terminated: bool) {
        debug!("Watchee terminated: [{}]", watchee.path());
        // When watchee is stopped it sends DeathWatchNotification to this RemoteWatcher,
        // which will propagate it to all watchers of this watchee.
        // addressTerminated case is already handled by the watcher itself in DeathWatch trait
        if !address_terminated {
            if let Some(watchers) = self.watching.get(&watchee) {
                let notification = DeathWatchNotification {
                    actor: watchee.clone(),
                    existence_confirmed,
                    address_terminated,
                };
                for watcher in watchers {
                    watcher.cast_system(notification.clone(), ActorRef::no_sender());
                }
            }
        }
        self.remove_watchee(&watchee);
    }

    fn receive_heartbeat_rsp(&mut self, context: &mut ActorContext, from: &Address, uid: i64) {
        if self.failure_detector.is_monitoring(from) {
            debug!("Received heartbeat rsp from [{}]", from);
        } else {
            debug!("Received first heartbeat rsp from [{}]", from);
        }
        if self.watchee_by_nodes.contains_key(from) && self.unreachable.contains(from).not() {
            if self.address_uids.get(from).map(|x| x != &uid).unwrap_or(true) {
                self.re_watch(context.myself().clone(), &from);
                self.address_uids.insert(from.clone(), uid);
                self.failure_detector.heartbeat(from.clone());
            }
        }
    }

    fn re_watch(&self, watcher: ActorRef, address: &Address) {
        if let Some(watchees) = self.watchee_by_nodes.get(address) {
            for watchee in watchees {
                debug!("Re-watch [{} -> {}]", watcher.path(), watchee.path());
                let watch = Watch {
                    watchee: watchee.clone(),
                    watcher: watcher.clone(),
                };
                watchee.cast_system(watch, ActorRef::no_sender());
            }
        }
    }
}