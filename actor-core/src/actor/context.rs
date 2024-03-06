use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::ops::Not;
use std::sync::Arc;

use anyhow::anyhow;
use arc_swap::Guard;
use tokio::runtime::Handle;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::{debug, error, warn};

use crate::{Actor, DynMessage, Message, MessageType, OrphanMessage};
use crate::actor::actor_system::ActorSystem;
use crate::actor::props::Props;
use crate::actor::state::ActorState;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_path::child_actor_path::ChildActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt};
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::function_ref::{FunctionRef, Inner};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::cell::Cell;
use crate::cell::envelope::Envelope;
use crate::ext::option_ext::OptionExt;
use crate::ext::random_name;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::execute::Execute;
use crate::message::failed::Failed;
use crate::message::terminate::Terminate;
use crate::message::terminated::Terminated;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
use crate::provider::ActorRefProvider;

pub trait Context: ActorRefFactory {
    fn myself(&self) -> &ActorRef;

    fn sender(&self) -> Option<&ActorRef>;

    fn children(&self) -> Vec<ActorRef>;

    fn child(&self, name: &str) -> Option<ActorRef>;

    fn parent(&self) -> Option<&ActorRef>;

    fn watch<T>(&mut self, terminated: T) where T: Terminated;

    fn unwatch(&mut self, subject: &ActorRef);

    fn is_watching(&self, subject: &ActorRef) -> bool;

    fn message_adapter<M>(&mut self, f: impl Fn(M) -> DynMessage + Send + Sync + 'static) -> ActorRef where M: OrphanMessage;
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
    pub(crate) stash: VecDeque<Envelope>,
    pub(crate) fut_handle: Vec<JoinHandle<()>>,
    pub(crate) system: ActorSystem,
    pub(crate) watching: HashMap<ActorRef, DynMessage>,
    pub(crate) watched_by: HashSet<ActorRef>,
    pub(crate) handle: Option<Handle>,
    pub(crate) stash_capacity: Option<usize>,
}

impl ActorRefFactory for ActorContext {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn provider_full(&self) -> Arc<ActorRefProvider> {
        self.system.provider_full()
    }

    fn provider(&self) -> Guard<Arc<ActorRefProvider>> {
        self.system.provider()
    }

    fn guardian(&self) -> LocalActorRef {
        self.system.guardian()
    }

    fn lookup_root(&self) -> ActorRef {
        self.myself().clone()
    }

    fn spawn(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        self.myself.local().unwrap()
            .attach_child(props, Some(name.into()), None, true)
            .map(|(actor, _)| actor)
    }

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        self.myself.local().unwrap()
            .attach_child(props, None, None, true)
            .map(|(actor, _)| actor)
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

    fn children(&self) -> Vec<ActorRef> {
        let myself = self.myself().local().unwrap();
        myself.underlying().children().iter().map(|c| c.value().clone()).collect()
    }

    fn child(&self, name: &str) -> Option<ActorRef> {
        let myself = self.myself().local().unwrap();
        myself.underlying().children().get(name).map(|c| c.value().clone())
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.myself().local().unwrap().cell.parent()
    }

    fn watch<T>(&mut self, terminated: T) where T: Terminated {
        let watchee = terminated.actor().clone();
        if &watchee != self.myself() {
            let message = DynMessage::user(terminated);
            match self.watching.get(&watchee) {
                None => {
                    let watch = Watch {
                        watchee: watchee.clone(),
                        watcher: self.myself.clone(),
                    };
                    watchee.cast_system(watch, ActorRef::no_sender());
                    self.watching.insert(watchee, message);
                }
                Some(previous) => {
                    warn!("drop previous watch message {} and insert new watch message {}", previous.name(), message.name());
                    self.watching.insert(watchee, message);
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
                self.provider()
                    .resolve_actor_ref_of_path(subject.path())
                    .cast_system(Unwatch { watchee, watcher }, ActorRef::no_sender());
            }
        }
    }

    fn is_watching(&self, subject: &ActorRef) -> bool {
        self.watching.contains_key(subject)
    }

    fn message_adapter<M>(&mut self, f: impl Fn(M) -> DynMessage + Send + Sync + 'static) -> ActorRef where M: OrphanMessage {
        let myself = self.myself.clone();
        self.add_function_ref(move |message, sender| {
            let DynMessage { name, ty: message_type, message: boxed } = message;
            let downcast_name = std::any::type_name::<M>();
            if matches!(message_type, MessageType::Orphan) {
                match boxed.into_any().downcast::<M>() {
                    Ok(message) => {
                        myself.tell(f(*message), sender);
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
    pub(crate) fn new(myself: ActorRef, system: ActorSystem, handle: Option<Handle>) -> Self {
        Self {
            state: ActorState::Init,
            myself,
            sender: None,
            stash: VecDeque::new(),
            fut_handle: vec![],
            system,
            watching: HashMap::new(),
            watched_by: HashSet::new(),
            handle,
            stash_capacity: None,
        }
    }

    pub fn stash<M>(&mut self, message: M) where M: Message {
        let sender = self.sender.clone();
        self.stash.push_back(Envelope::new(DynMessage::user(message), sender));
        if let Some(stash_capacity) = self.stash_capacity {
            if self.stash.len() > stash_capacity {
                if let Some(oldest) = self.stash.pop_front() {
                    let name = oldest.name();
                    warn!("stash buffer reach max size {}, drop oldest message {}", stash_capacity, name);
                }
            }
        }
    }

    pub fn unstash(&mut self) -> bool {
        if let Some(envelope) = self.stash.pop_front() {
            let message = envelope.message;
            let sender = envelope.sender;
            self.myself.tell(message, sender);
            return true;
        }
        return false;
    }

    pub fn unstash_all(&mut self) -> bool {
        if self.stash.is_empty() {
            return false;
        }
        for envelope in self.stash.drain(..) {
            let message = envelope.message;
            let sender = envelope.sender;
            self.myself.tell(message, sender);
        }
        return true;
    }

    pub(crate) fn terminate(&mut self) {
        self.state = ActorState::Terminating;
        let children = self.children();
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
        if self.myself.local().unwrap().children().get(watching_actor.path().name()).is_some() {
            self.handle_child_terminated(watching_actor);
        }
    }

    pub(crate) fn handle_child_terminated(&mut self, actor: ActorRef) {
        let myself = self.myself.local().unwrap();
        myself.cell.remove_child(actor.path().name());
        if matches!(self.state, ActorState::Terminating) && myself.cell.children().is_empty() {
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

    pub fn spawn_fut<F>(&mut self, future: F) -> AbortHandle
        where
            F: Future<Output=()> + Send + 'static,
    {
        let handle = match &self.handle {
            None => {
                self.system.handle().spawn(future)
            }
            Some(handle) => {
                handle.spawn(future)
            }
        };
        let abort_handle = handle.abort_handle();
        self.fut_handle.push(handle);
        abort_handle
    }

    pub(crate) fn remove_finished_tasks(&mut self) {
        if !self.fut_handle.is_empty() {
            self.fut_handle.retain(|t| !t.is_finished());
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

    pub fn execute<F, A>(&self, f: F)
        where
            F: FnOnce(&mut ActorContext, &mut A) -> anyhow::Result<()> + Send + 'static,
            A: Actor {
        let execute = Execute {
            closure: Box::new(f),
        };
        self.myself.cast_ns(execute);
    }

    pub(crate) fn handle_invoke_failure(&mut self, child: ActorRef, name: &str, error: anyhow::Error) {
        error!("{} handle message error {:?}", name, error);
        self.state = ActorState::Suspend;
        self.parent().foreach(move |p| {
            p.cast_system(Failed { child }, ActorRef::no_sender());
        });
    }
}