use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::ops::{Deref, DerefMut, Not};
use std::sync::Arc;

use crate::actor::actor_system::ActorSystem;
use crate::actor::props::Props;
use crate::actor::state::ActorState;
use crate::actor::watching::Watching;
use crate::actor_path::child_actor_path::ChildActorPath;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::function_ref::{FunctionRef, FunctionRefInner};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::cell::envelope::Envelope;
use crate::event::address_terminated_topic::AddressTerminatedTopic;
use crate::ext::random_name;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::failed::Failed;
use crate::message::task_finish::TaskFinish;
use crate::message::terminate::Terminate;
use crate::message::terminated::Terminated;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
use crate::message::{DynMessage, Message};
use crate::provider::ActorRefProvider;
use ahash::{HashMap, HashSet, RandomState};
use anyhow::{anyhow, bail};
use dashmap::DashMap;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::{debug, error, warn};

macro_rules! local {
    ($actor_ref: expr) => {
        $actor_ref.local().expect("ActorRef is not LocalActorRef")
    };
}

pub trait ActorContext: ActorRefFactory + Sized {
    fn new(system: ActorSystem, myself: ActorRef) -> Self;

    fn context(&self) -> &Context;

    fn context_mut(&mut self) -> &mut Context;
}

#[derive(Debug)]
pub struct Context {
    pub(crate) state: ActorState,
    pub(crate) myself: ActorRef,
    pub(crate) function_refs: HashMap<String, FunctionRef>,
    pub(crate) stash_capacity: Option<usize>,
    pub(crate) stash: VecDeque<Envelope>,
    pub(crate) task_id: usize,
    pub(crate) abort_handles: HashMap<String, AbortHandle>,
    pub(crate) system: ActorSystem,
    pub(crate) watching: Watching,
    pub(crate) watched_by: HashSet<ActorRef>,
}

impl ActorRefFactory for Context {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn provider(&self) -> &ActorRefProvider {
        self.system.provider()
    }

    fn guardian(&self) -> &LocalActorRef {
        self.system.guardian()
    }

    fn lookup_root(&self) -> &dyn TActorRef {
        self.myself()
    }

    fn spawn(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        local!(self.myself).attach_child(props, self.system.clone(), self.provider(), Some(name.into()), None)
    }

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef> {
        if !matches!(self.state, ActorState::Init | ActorState::Started) {
            return Err(anyhow!(
                "cannot spawn child actor while parent actor {} is terminating",
                self.myself
            ));
        }
        local!(self.myself).attach_child(props, self.system.clone(), self.provider(), None, None)
    }

    fn stop(&self, actor: &ActorRef) {
        actor.cast_ns(Terminate);
    }
}

impl ActorContext for Context {
    fn new(system: ActorSystem, myself: ActorRef) -> Self {
        Context {
            state: ActorState::Init,
            myself,
            function_refs: Default::default(),
            stash_capacity: None,
            stash: Default::default(),
            task_id: 0,
            abort_handles: Default::default(),
            system,
            watching: Default::default(),
            watched_by: Default::default(),
        }
    }

    fn context(&self) -> &Context {
        self
    }

    fn context_mut(&mut self) -> &mut Context {
        &mut self
    }
}

impl Context {
    pub fn stash<M>(&mut self, message: M, sender: Option<ActorRef>)
    where
        M: Message + 'static,
    {
        self.stash.push_back(Envelope::new(message, sender));
        if let Some(stash_capacity) = self.stash_capacity {
            if self.stash.len() > stash_capacity {
                if let Some(oldest) = self.stash.pop_front() {
                    let name = oldest.name();
                    warn!(
                        "stash buffer reach max size {}, drop oldest message {}",
                        stash_capacity, name
                    );
                }
            }
        }
    }

    pub fn unstash(&mut self) -> bool {
        if let Some(Envelope { message, sender }) = self.stash.pop_front() {
            self.myself.tell(message, sender);
            return true;
        }
        false
    }

    pub fn unstash_all(&mut self) -> bool {
        if self.stash.is_empty() {
            return false;
        }
        for Envelope { message, sender } in self.stash.drain(..) {
            self.myself.tell(message, sender);
        }
        true
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
            let notification = DeathWatchNotification {
                actor: self.myself.clone(),
                existence_confirmed: true,
                address_terminated: false,
            };
            parent.cast_ns(notification);
        }
        self.tell_watchers_we_died();
        self.unwatch_watched_actors();
        self.state = ActorState::CanTerminate;
    }

    pub(crate) fn watched_actor_terminated(
        &mut self,
        actor: ActorRef,
        existence_confirmed: bool,
        address_terminated: bool,
    ) {
        debug!("{} watched actor {} terminated", self.myself, actor);
        if self.watching.contains_key(&actor) {
            let optional_message = self
                .maintain_address_terminated_subscription(Some(&actor), |ctx| {
                    ctx.watching.remove(&actor).unwrap()
                });
            if !matches!(self.state, ActorState::Terminating) {
                let terminated = Terminated {
                    actor: actor.clone(),
                    existence_confirmed,
                    address_terminated,
                };
                self.myself.tell(optional_message(terminated), None);
            }
        }
        if self.children().contains_key(actor.path().name()) {
            self.handle_child_terminated(actor);
        }
    }

    pub(crate) fn handle_child_terminated(&mut self, actor: ActorRef) {
        let myself = local!(self.myself);
        myself.remove_child(actor.path().name());
        if matches!(self.state, ActorState::Terminating) && myself.children.is_empty() {
            self.finish_terminate();
        }
    }

    fn tell_watchers_we_died(&mut self) {
        let (local_watchers, remote_watchers): (Vec<_>, Vec<_>) = self
            .watched_by
            .iter()
            .partition(|w| &self.system().address() == w.path().address());
        remote_watchers
            .iter()
            .chain(&local_watchers)
            .for_each(|watcher| {
                if self.myself.parent().map(|p| &p != watcher).unwrap_or(true) {
                    let myself = self.myself.clone();
                    debug!("{} tell watcher {} we died", myself, watcher);
                    let notification = DeathWatchNotification {
                        actor: myself,
                        existence_confirmed: true,
                        address_terminated: false,
                    };
                    watcher.cast_system(notification, ActorRef::no_sender());
                }
            });
        self.maintain_address_terminated_subscription(None, |ctx| {
            ctx.watched_by.clear();
        });
    }

    fn unwatch_watched_actors(&mut self) {
        self.maintain_address_terminated_subscription(None, |ctx| {
            ctx.watching.drain().for_each(|(actor, _)| {
                let watchee = actor.clone();
                let watcher = ctx.myself.clone();
                actor.cast_ns(Unwatch { watchee, watcher });
            });
        });
    }

    pub fn spawn_fut<F>(
        &mut self,
        name: impl Into<String>,
        future: F,
    ) -> anyhow::Result<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_inner(name.into(), future)
    }

    #[cfg(feature = "tokio-tracing")]
    pub(crate) fn spawn_inner<F>(
        &mut self,
        name: String,
        future: F,
    ) -> anyhow::Result<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let name = format!("{}-{}", name, self.task_id);
        self.task_id = self.task_id.wrapping_add(1);
        let myself = self.myself.clone();
        let task_name = name.clone();
        let handle = tokio::task::Builder::new().name(&name).spawn(async move {
            let output = future.await;
            myself.cast_ns(TaskFinish { name: task_name });
            output
        })?;
        let abort_handle = handle.abort_handle();
        self.abort_handles.insert(name, abort_handle);
        Ok(handle)
    }

    #[cfg(not(feature = "tokio-tracing"))]
    pub(crate) fn spawn_inner<F>(
        &mut self,
        name: String,
        future: F,
    ) -> anyhow::Result<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let name = format!("{}-{}", name, self.task_id);
        self.task_id = self.task_id.wrapping_add(1);
        let myself = self.myself.clone();
        let task_name = name.clone();
        let handle = tokio::spawn(async move {
            let output = future.await;
            myself.cast(TaskFinish { name: task_name });
            output
        });
        let abort_handle = handle.abort_handle();
        self.abort_handles.insert(name, abort_handle);
        Ok(handle)
    }

    pub(crate) fn add_function_ref<F>(&mut self, transform: F, name: Option<String>) -> FunctionRef
    where
        F: Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static,
    {
        let mut n = random_name("$$".to_string());
        if let Some(name) = name {
            n.push_str(&*format!("-{}", name));
        }
        let child_path = ChildActorPath::new(self.myself.path().clone(), n, ActorPath::new_uid());
        let name = child_path.name().clone();
        let function_ref = FunctionRef::new(child_path, transform);
        self.function_refs.insert(name, function_ref.clone());
        function_ref
    }

    pub(crate) fn remove_function_ref(&self, name: &str) -> bool {
        self.myself
            .local()
            .unwrap()
            .underlying()
            .remove_function_ref(name)
            .is_some()
    }

    pub(crate) fn handle_invoke_failure(
        &mut self,
        actor: &str,
        message: &str,
        error: anyhow::Error,
    ) {
        error!("{} handle message {} error {:?}", actor, message, error);
        self.state = ActorState::Suspend;
        let myself = self.myself().local().unwrap();
        for child in self.children() {
            child.value().suspend();
        }
        if let Some(parent) = self.parent() {
            parent.cast_ns(Failed { child, error });
        }
    }

    pub(crate) fn maintain_address_terminated_subscription<F, T>(
        &mut self,
        change: Option<&ActorRef>,
        block: F,
    ) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        fn is_non_local(system: &ActorSystem, actor: Option<&ActorRef>) -> bool {
            match actor {
                None => true,
                Some(actor) => actor.path().address() != system.address(),
            }
        }
        fn has_non_local_address(ctx: &Context) -> bool {
            ctx.watching
                .keys()
                .any(|w| is_non_local(ctx.system(), Some(w)))
                || ctx
                    .watched_by
                    .iter()
                    .any(|w| is_non_local(ctx.system(), Some(w)))
        }
        if is_non_local(self.system(), change) {
            let had = has_non_local_address(self);
            let result = block(self);
            let has = has_non_local_address(self);
            if had && !has {
                AddressTerminatedTopic::get(self.system()).unsubscribe(self.myself());
            } else {
                AddressTerminatedTopic::get(self.system()).subscribe(self.myself().clone());
            }
            result
        } else {
            block(self)
        }
    }


    pub fn myself(&self) -> &ActorRef {
        &self.myself
    }

    pub fn children(&self) -> &DashMap<String, ActorRef, RandomState> {
        &local!(self.myself).children
    }

    pub fn child(&self, name: &str) -> Option<ActorRef> {
        local!(self.myself).children.get(name).map(|c| c.value().clone())
    }

    pub fn parent(&self) -> Option<&ActorRef> {
        local!(self.myself).parent.as_ref()
    }

    fn watch(&mut self, subject: &ActorRef) -> anyhow::Result<()> {
        if subject != self.myself() {
            match self.watching.get(&subject) {
                None => {
                    self.maintain_address_terminated_subscription(Some(&subject), |ctx| {
                        let watch = Watch {
                            watchee: subject.clone(),
                            watcher: ctx.myself.clone(),
                        };
                        subject.cast_ns(watch);
                        ctx.watching.insert(subject.clone(), None);
                    });
                }
                Some(_) => {
                    bail!("duplicate watch {}, you should unwatch it first.", subject);
                }
            }
        } else {
            bail!("cannot watch self");
        }
        Ok(())
    }

    fn watch_with(&mut self, subject: &ActorRef, msg: DynMessage) -> anyhow::Result<()> {
        if subject != self.myself() {
            match self.watching.get(&subject) {
                None => {
                    self.maintain_address_terminated_subscription(Some(&subject), |ctx| {
                        let watch = Watch {
                            watchee: subject.clone(),
                            watcher: ctx.myself.clone(),
                        };
                        subject.cast_ns(watch);
                        ctx.watching.insert(subject.clone(), Some(msg));
                    });
                }
                Some(_) => {
                    bail!("duplicate watch {}, you should unwatch it first.", subject);
                }
            }
        } else {
            bail!("cannot watch self");
        }
        Ok(())
    }

    fn unwatch(&mut self, subject: &ActorRef) {
        if &self.myself != subject && self.watching.contains_key(subject) {
            let watchee = subject.clone();
            let watcher = self.myself.clone();
            let unwatch = Unwatch { watchee, watcher };
            subject.cast_system(unwatch, ActorRef::no_sender());
            self.maintain_address_terminated_subscription(Some(subject), |ctx| {
                ctx.watching.remove(subject);
            });
        }
    }

    fn is_watching(&self, subject: &ActorRef) -> bool {
        self.watching.contains_key(subject)
    }
}
