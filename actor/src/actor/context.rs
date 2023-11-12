use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::ops::Not;
use std::pin::Pin;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use anyhow::{anyhow, Ok};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::error;

use crate::actor::{Actor, DynamicMessage, Message, UserDelegate};
use crate::actor_path::{ActorPath, ChildActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::{check_name, random_actor_name};
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::terminate::Terminate;
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
    fn watch(&self, subject: &ActorRef) -> anyhow::Result<()>;
    fn watch_with(&self, subject: &ActorRef, message: DynamicMessage);
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
    pub(crate) tasks: Vec<JoinHandle<()>>,
    pub(crate) system: ActorSystem,
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

    fn watch(&self, subject: &ActorRef) -> anyhow::Result<()> {
        Ok(())
    }

    fn watch_with(&self, subject: &ActorRef, message: DynamicMessage) {
        todo!()
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

    pub(crate) fn handle_terminate(&mut self) {
        self.state = ActorState::Terminating;
        let children: Vec<ActorRef> = self.children().values().cloned().collect();
        if children.is_empty().not() {
            for child in &children {
                self.stop(child);
            }
        } else {
            self.finish_terminated();
        }
    }

    pub(crate) fn finish_terminated(&mut self) {
        if let Some(parent) = self.parent() {
            parent.cast_system(DeathWatchNotification(self.myself.clone().into()), Some(self.myself.clone()));
        }
        self.state = ActorState::CanTerminate;
    }

    pub(crate) fn watched_actor_terminated(&mut self, actor: ActorRef) {
        if self.children().get(actor.path().name()).is_some() {
            self.handle_child_terminated(actor);
        }
    }

    pub(crate) fn handle_child_terminated(&mut self, actor: ActorRef) {
        let mut children = self.children_mut();
        children.remove(actor.path().name());
        if matches!(self.state, ActorState::Terminating) && children.is_empty() {
            drop(children);
            self.finish_terminated();
        }
    }

    pub fn spawn<F>(&mut self, future: F)
        where
            F: Future<Output=()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        self.tasks.push(handle);
    }

    pub(crate) fn remove_finished_task(&mut self) {
        if !self.tasks.is_empty() {
            self.tasks.retain(|t| !t.is_finished());
        }
    }
}

pub(crate) enum ActorThreadPoolMessage {
    SpawnActor(
        Box<dyn FnOnce(&mut FuturesUnordered<Pin<Box<dyn Future<Output=()>>>>) + Send + 'static>,
    ),
}

#[derive(Debug)]
pub(crate) struct ActorThreadPool {
    pub(crate) sender: crossbeam::channel::Sender<ActorThreadPoolMessage>,
    pub(crate) handles: Vec<std::thread::JoinHandle<()>>,
}

impl ActorThreadPool {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam::channel::bounded::<ActorThreadPoolMessage>(100);
        let mut handles = vec![];
        let cpus = num_cpus::get();
        for cpu in 1..=cpus {
            let rx = rx.clone();
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .thread_name_fn(move || format!("actor_dispatcher_{}", cpu))
                    .enable_all()
                    .build()
                    .expect(&format!("failed to build local set runtime {}", cpu));
                rt.block_on(async {
                    let mut futures = FuturesUnordered::new();
                    let (m_tx, mut m_rx) = tokio::sync::mpsc::unbounded_channel();
                    tokio::task::spawn_blocking(move || {
                        for message in rx {
                            match message {
                                ActorThreadPoolMessage::SpawnActor(spawn_fn) => {
                                    if let Err(_) = m_tx.send(spawn_fn) {
                                        error!("send spawn actor message error, receiver closed");
                                    }
                                }
                            }
                        }
                    });
                    loop {
                        tokio::select! {
                            Some(spawn_fn) = m_rx.recv() => {
                                spawn_fn(&mut futures);
                            }
                            _ = futures.next(), if !futures.is_empty() => {

                            }
                        }
                    }
                });
            });
            handles.push(handle);
        }
        Self {
            sender: tx,
            handles,
        }
    }
}
