use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::error;

use crate::actor::Actor;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::ActorCell;
use crate::message::ActorMessage;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider};
use crate::state::ActorState;
use crate::system::ActorSystem;

pub trait Context: ActorRefFactory {
    fn myself(&self) -> &ActorRef;
    fn sender(&self) -> Option<&ActorRef>;
    fn children(&self) -> RwLockReadGuard<BTreeMap<String, ActorRef>>;
    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>>;
    fn child(&self, name: &String) -> Option<ActorRef>;
    fn parent(&self) -> Option<&ActorRef>;
    fn watch(&self, subject: &ActorRef);
    fn watch_with(&self, subject: &ActorRef, message: ActorMessage);
    fn unwatch(&self, subject: &ActorRef);
}

impl<T: ?Sized> ContextExt for T where T: Context {}

pub trait ContextExt: Context {
    fn forward(&self, to: &ActorRef, message: ActorMessage) {
        to.tell(message, self.sender().cloned())
    }
}

#[derive(Debug)]
pub struct ActorContext<T> where T: Actor {
    pub(crate) state: ActorState,
    pub(crate) cell: ActorCell,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) stash: VecDeque<(T::M, Option<ActorRef>)>,
    pub(crate) tasks: Vec<JoinHandle<()>>,
    pub(crate) system: ActorSystem,
}

impl<A> ActorRefFactory for ActorContext<A> where A: Actor {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn provider(&self) -> &ActorRefProvider {
        self.system.provider()
    }

    fn guardian(&self) -> &ActorRef {
        todo!()
    }

    fn lookup_root(&self) -> ActorRef {
        todo!()
    }

    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, name: Option<String>) -> anyhow::Result<ActorRef> where T: Actor {
        let supervisor = self.cell.myself();
        todo!()
    }

    fn stop(&self, actor: &ActorRef) {
        todo!()
    }
}

impl<A> Context for ActorContext<A> where A: Actor {
    fn myself(&self) -> &ActorRef {
        self.cell.myself()
    }

    fn sender(&self) -> Option<&ActorRef> {
        self.sender.as_ref()
    }

    fn children(&self) -> RwLockReadGuard<BTreeMap<String, ActorRef>> {
        self.cell.children().read().unwrap()
    }

    fn children_mut(&self) -> RwLockWriteGuard<BTreeMap<String, ActorRef>> {
        self.cell.children().write().unwrap()
    }

    fn child(&self, name: &String) -> Option<ActorRef> {
        self.cell.children().read().unwrap().get(name).cloned()
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.cell.parent().map(|p| p.myself())
    }

    fn watch(&self, subject: &ActorRef) {
        todo!()
    }

    fn watch_with(&self, subject: &ActorRef, message: ActorMessage) {
        todo!()
    }

    fn unwatch(&self, subject: &ActorRef) {
        todo!()
    }
}

impl<A> ActorContext<A> where A: Actor {
    pub fn stash(&mut self, message: A::M) {
        let sender = self.sender.clone();
        self.stash.push_back((message, sender));
    }

    pub fn unstash(&mut self) -> bool {
        if let Some((message, sender)) = self.stash.pop_front() {
            // self.myself.tell(message, sender);
            return true;
        }
        return false;
    }

    pub fn unstash_all(&mut self) -> bool {
        if self.stash.is_empty() {
            return false;
        }
        for (message, sender) in self.stash.drain(0..) {
            // self.myself.tell(message, sender);
        }
        return true;
    }
}

pub(crate) enum ActorThreadPoolMessage {
    SpawnActor(Box<dyn FnOnce(&mut FuturesUnordered<Pin<Box<dyn Future<Output=()>>>>) + Send + 'static>)
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
                    .thread_name_fn(move || {
                        format!("actor_dispatcher_{}", cpu)
                    })
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