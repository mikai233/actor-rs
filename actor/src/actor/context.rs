use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::ops::Not;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use anyhow::anyhow;
use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::actor::{Actor, DynamicMessage, Message, UserDelegate};
use crate::actor_path::{ActorPath, ChildActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::{check_name, random_actor_name};
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::terminate::Terminate;
use crate::message::terminated::WatchTerminated;
use crate::message::watch::Watch;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::state::ActorState;
use crate::system::ActorSystem;

pub trait Context: ActorRefFactory {
    fn myself(&self) -> &ActorRef;
    fn sender(&self) -> Option<&ActorRef>;
    fn children(&self) -> RwLockReadGuard<BTreeMap<String, ActorRef>>;
    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>>;
    fn child(&self, name: &String) -> Option<ActorRef>;
    fn parent(&self) -> Option<&ActorRef>;
    fn watch<T>(&mut self, terminate: T) where T: WatchTerminated;
    fn unwatch(&self, subject: &ActorRef);
}

impl<T: ?Sized> ContextExt for T where T: Context {}

pub trait ContextExt: Context {
    fn forward(&self, to: &ActorRef, message: DynamicMessage) {
        to.tell(message, self.sender().cloned())
    }
}

#[derive(Debug)]
pub struct ActorContext {
    pub(crate) state: ActorState,
    pub(crate) myself: ActorRef,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) stash: VecDeque<(DynamicMessage, Option<ActorRef>)>,
    pub(crate) async_tasks: Vec<JoinHandle<()>>,
    pub(crate) system: ActorSystem,
    pub(crate) watching: HashMap<ActorRef, DynamicMessage>,
    pub(crate) watched_by: HashSet<ActorRef>,
}

impl ActorRefFactory for ActorContext {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn provider(&self) -> ActorRefProvider {
        self.system.provider()
    }

    fn guardian(&self) -> LocalActorRef {
        self.system.guardian()
    }

    fn lookup_root(&self) -> ActorRef {
        self.myself().clone()
    }

    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        name: Option<String>,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor,
    {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        let supervisor = &self.myself;
        if let Some(name) = &name {
            check_name(name)?;
        }
        let name = name.unwrap_or_else(random_actor_name);
        let path =
            ChildActorPath::new(supervisor.path().clone(), name, ActorPath::new_uid()).into();
        self.provider()
            .actor_of(actor, arg, props, supervisor, path)
    }

    fn stop(&self, actor: &ActorRef) {
        let sender = self.myself().clone();
        actor.cast_system(Terminate, Some(sender));
    }
}

impl Context for ActorContext {
    fn myself(&self) -> &ActorRef {
        &self.myself
    }

    fn sender(&self) -> Option<&ActorRef> {
        self.sender.as_ref()
    }

    fn children(&self) -> RwLockReadGuard<BTreeMap<String, ActorRef>> {
        let myself = self.myself().local_or_panic();
        myself.cell.children().read().unwrap()
    }

    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>> {
        let myself = self.myself().local_or_panic();
        myself.cell.children().write().unwrap()
    }

    fn child(&self, name: &String) -> Option<ActorRef> {
        self.children().get(name).cloned()
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.myself().local_or_panic().cell.parent()
    }

    fn watch<T>(&mut self, terminate: T) where T: WatchTerminated {
        let system = self.system();
        let actor = terminate.actor(system);
        let message = DynamicMessage::user(terminate);
        if actor != self.myself {
            match self.watching.get(&actor) {
                None => {
                    let watch = Watch {
                        watchee: actor.clone().into(),
                        watcher: self.myself.clone().into(),
                    };
                    actor.cast_system(watch, Some(self.myself.clone()));
                    self.watching.insert(actor, message);
                }
                Some(previous) => {
                    warn!("drop previous watch message {} and insert new watch message {}", previous.name(), message.name());
                    self.watching.insert(actor, message);
                }
            }
        }
    }

    fn unwatch(&self, subject: &ActorRef) {}
}

impl ActorContext {
    pub fn stash<M>(&mut self, message: M) where M: Message {
        let sender = self.sender.clone();
        let message = UserDelegate::new(message);
        self.stash.push_back((message.into(), sender));
    }

    pub fn unstash(&mut self) -> bool {
        if let Some((message, sender)) = self.stash.pop_front() {
            self.myself.tell(message, sender);
            return true;
        }
        return false;
    }

    pub fn unstash_all(&mut self) -> bool {
        if self.stash.is_empty() {
            return false;
        }
        for (message, sender) in self.stash.drain(0..) {
            self.myself.tell(message, sender);
        }
        return true;
    }

    pub(crate) fn terminate(&mut self) {
        self.state = ActorState::Terminating;
        let children: Vec<ActorRef> = self.children().values().cloned().collect();
        if children.is_empty().not() {
            for child in &children {
                self.stop(child);
            }
        } else {
            self.finish_terminate();
        }
    }

    pub(crate) fn finish_terminate(&mut self) {
        if let Some(parent) = self.parent() {
            parent.cast_system(DeathWatchNotification(self.myself.clone().into()), Some(self.myself.clone()));
        }
        self.state = ActorState::CanTerminate;
    }

    pub(crate) fn watched_actor_terminated(&mut self, actor: ActorRef) {
        if let Some(watch_terminated) = self.watching.remove(&actor) {
            if !matches!(self.state, ActorState::Terminating) {
                self.myself.tell(watch_terminated, None);
            }
        }
        if self.children().get(actor.path().name()).is_some() {
            self.handle_child_terminated(actor);
        }
    }

    pub(crate) fn handle_child_terminated(&mut self, actor: ActorRef) {
        let mut children = self.children_mut();
        children.remove(actor.path().name());
        if matches!(self.state, ActorState::Terminating) && children.is_empty() {
            drop(children);
            self.finish_terminate();
        }
    }

    pub fn spawn<F>(&mut self, future: F)
        where
            F: Future<Output=()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        self.async_tasks.push(handle);
    }

    pub(crate) fn remove_finished_tasks(&mut self) {
        if !self.async_tasks.is_empty() {
            self.async_tasks.retain(|t| !t.is_finished());
        }
    }
}