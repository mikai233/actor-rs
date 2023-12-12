use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::ops::Not;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use anyhow::anyhow;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

use crate::{Actor, DynMessage, Message, MessageType, UntypedMessage, UserDelegate};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_path::child_actor_path::ChildActorPath;
use crate::actor::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::actor_system::ActorSystem;
use crate::actor::cell::Cell;
use crate::actor::function_ref::{FunctionRef, Inner};
use crate::actor::local_ref::LocalActorRef;
use crate::actor::props::Props;
use crate::actor::state::ActorState;
use crate::ext::option_ext::OptionExt;
use crate::ext::random_name;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::execute::Execute;
use crate::message::failed::Failed;
use crate::message::terminate::Terminate;
use crate::message::terminated::WatchTerminated;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;

pub trait Context: ActorRefFactory {
    fn myself(&self) -> &ActorRef;
    fn sender(&self) -> Option<&ActorRef>;
    fn children(&self) -> RwLockReadGuard<BTreeMap<String, ActorRef>>;
    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>>;
    fn child(&self, name: &String) -> Option<ActorRef>;
    fn parent(&self) -> Option<&ActorRef>;
    fn watch<T>(&mut self, terminate: T) where T: WatchTerminated;
    fn unwatch(&mut self, subject: &ActorRef);
    fn is_watching(&self, subject: &ActorRef) -> bool;
    fn message_adapter<M>(&mut self, f: impl Fn(M) -> anyhow::Result<DynMessage> + Send + Sync + 'static) -> ActorRef
        where
            M: UntypedMessage;
}

impl<T: ?Sized> ContextExt for T where T: Context {}

pub trait ContextExt: Context {
    fn forward(&self, to: &ActorRef, message: DynMessage) {
        to.tell(message, self.sender().cloned())
    }
}

#[derive(Debug)]
pub struct ActorContext {
    pub(crate) state: ActorState,
    pub(crate) myself: ActorRef,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) stash: VecDeque<(DynMessage, Option<ActorRef>)>,
    pub(crate) async_tasks: Vec<JoinHandle<()>>,
    pub(crate) system: ActorSystem,
    pub(crate) watching: HashMap<ActorRef, DynMessage>,
    pub(crate) watched_by: HashSet<ActorRef>,
}

impl ActorRefFactory for ActorContext {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn provider(&self) -> Arc<ActorRefProvider> {
        self.system.provider()
    }

    fn guardian(&self) -> LocalActorRef {
        self.system.guardian()
    }

    fn lookup_root(&self) -> ActorRef {
        self.myself().clone()
    }

    fn spawn_actor(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        self.myself.local().unwrap().attach_child(props, Some(name.into()), true).map(|(actor, _)| actor)
    }

    fn spawn_anonymous_actor(&self, props: Props) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        self.myself.local().unwrap().attach_child(props, None, true).map(|(actor, _)| actor)
    }

    fn stop(&self, actor: &ActorRef) {
        actor.cast_system(Terminate, ActorRef::no_sender());
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
        let myself = self.myself().local().unwrap();
        myself.cell.children().read().unwrap()
    }

    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>> {
        let myself = self.myself().local().unwrap();
        myself.cell.children().write().unwrap()
    }

    fn child(&self, name: &String) -> Option<ActorRef> {
        self.children().get(name).cloned()
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.myself().local().unwrap().cell.parent()
    }

    fn watch<T>(&mut self, terminate: T) where T: WatchTerminated {
        let watch_actor = terminate.watch_actor().clone();
        if &watch_actor != self.myself() {
            let message = DynMessage::user(terminate);
            match self.watching.get(&watch_actor) {
                None => {
                    let watch = Watch {
                        watchee: watch_actor.clone(),
                        watcher: self.myself.clone(),
                    };
                    watch_actor.cast_system(watch, ActorRef::no_sender());
                    self.watching.insert(watch_actor, message);
                }
                Some(previous) => {
                    warn!("drop previous watch message {} and insert new watch message {}", previous.name(), message.name());
                    self.watching.insert(watch_actor, message);
                }
            }
        }
    }

    fn unwatch(&mut self, subject: &ActorRef) {
        let myself = self.myself();
        if myself != subject && self.watching.contains_key(subject) {
            let watchee = subject.clone();
            let watcher = myself.clone();
            if self.myself.path().address() == subject.path().address() {
                subject.cast_system(Unwatch { watchee, watcher }, ActorRef::no_sender());
            } else {
                self.provider().resolve_actor_ref_of_path(subject.path()).cast_system(Unwatch { watchee, watcher }, ActorRef::no_sender());
            }
        }
    }

    fn is_watching(&self, subject: &ActorRef) -> bool {
        self.watching.contains_key(subject)
    }

    fn message_adapter<M>(&mut self, f: impl Fn(M) -> anyhow::Result<DynMessage> + Send + Sync + 'static) -> ActorRef
        where
            M: UntypedMessage {
        let myself = self.myself.clone();
        self.add_function_ref(move |message, sender| {
            let DynMessage { name, message_type, boxed } = message;
            let downcast_name = std::any::type_name::<M>();
            if matches!(message_type, MessageType::Untyped) {
                match boxed.into_any().downcast::<M>() {
                    Ok(message) => {
                        match f(*message) {
                            Ok(t) => {
                                myself.tell(t, sender);
                            }
                            Err(error) => {
                                error!("message {} transform error {:?}", downcast_name, error);
                            }
                        }
                    }
                    Err(_) => {
                        error!("message {} cannot downcast to {}", name, downcast_name);
                    }
                }
            }
        }, None).into()
    }
}

impl ActorContext {
    pub(crate) fn new(myself: ActorRef, system: ActorSystem) -> Self {
        Self {
            state: ActorState::Init,
            myself,
            sender: None,
            stash: Default::default(),
            async_tasks: vec![],
            system,
            watching: Default::default(),
            watched_by: Default::default(),
        }
    }
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
        for (message, sender) in self.stash.drain(..) {
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
            parent.cast_system(DeathWatchNotification(self.myself.clone()), ActorRef::no_sender());
        }
        self.tell_watchers_we_died();
        self.unwatch_watched_actors();
        self.state = ActorState::CanTerminate;
    }

    pub(crate) fn watched_actor_terminated(&mut self, watching_actor: ActorRef) {
        debug!("{} watched actor {} terminated", self.myself, watching_actor);
        if let Some(watch_terminated) = self.watching.remove(&watching_actor) {
            if !matches!(self.state, ActorState::Terminating) {
                self.myself.tell(watch_terminated, None);
            }
        }
        if self.children().get(watching_actor.path().name()).is_some() {
            self.handle_child_terminated(watching_actor);
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

    fn tell_watchers_we_died(&mut self) {
        self.watched_by.drain().for_each(|watcher| {
            if self.myself.parent().map(|p| *p != watcher).unwrap_or(true) {
                let myself = self.myself.clone();
                debug!("{} tell watcher {} we died", myself, watcher);
                watcher.cast_system(DeathWatchNotification(myself), ActorRef::no_sender());
            }
        });
    }

    fn unwatch_watched_actors(&mut self) {
        self.watching.drain().for_each(|(actor, _)| {
            let watchee = actor.clone();
            let watcher = self.myself.clone();
            actor.cast_system(Unwatch { watchee, watcher }, ActorRef::no_sender());
        })
    }

    pub fn spawn<F>(&mut self, future: F)
        where
            F: Future<Output=()> + Send + 'static,
    {
        let handle = self.system.spawn(future);
        self.async_tasks.push(handle);
    }

    pub(crate) fn remove_finished_tasks(&mut self) {
        if !self.async_tasks.is_empty() {
            self.async_tasks.retain(|t| !t.is_finished());
        }
    }

    pub(crate) fn add_function_ref<F>(&self, f: F, name: Option<String>) -> FunctionRef
        where
            F: Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static
    {
        let mut n = random_name("$$".to_string());
        if let Some(name) = name {
            n.push_str(&*format!("-{}", name));
        }
        let child_path = ChildActorPath::new(self.myself.path().clone(), n, ActorPath::new_uid());
        let name = child_path.name().clone();
        let inner = Inner {
            system: self.system.clone(),
            path: child_path.into(),
            message_handler: Arc::new(Box::new(f)),
        };
        let function_ref = FunctionRef {
            inner: inner.into(),
        };
        self.myself.local().unwrap().underlying().add_function_ref(name, function_ref.clone());
        function_ref
    }

    pub(crate) fn remove_function_ref(&self, name: &str) -> bool {
        self.myself.local().unwrap().underlying().remove_function_ref(name).is_some()
    }

    pub fn execute<F, A>(&self, f: F) where F: FnOnce(&mut ActorContext, &mut A) -> anyhow::Result<()> + Send + 'static, A: Actor {
        let execute = Execute {
            closure: Box::new(f),
        };
        self.myself.cast(execute, ActorRef::no_sender());
    }

    pub(crate) fn handle_invoke_failure(&mut self, child: ActorRef, error: anyhow::Error) {
        self.state = ActorState::Suspend;
        self.parent().foreach(move |p| {
            p.cast_system(Failed { child, error: error.to_string() }, ActorRef::no_sender());
        });
    }
}