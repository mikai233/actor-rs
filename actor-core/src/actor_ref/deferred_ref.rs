use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::error::Elapsed;

use actor_derive::AsAny;

use crate::actor::actor_selection::ActorSelection;
use crate::actor_path::ActorPath;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::{get_child_default, ActorRef, TActorRef};
use crate::message::DynMessage;

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct DeferredActorRef(Arc<DeferredActorRefInner>);

pub struct DeferredActorRefInner {
    path: ActorPath,
    parent: ActorRef,
    sender: Sender<DynMessage>,
}

impl Debug for DeferredActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredActorRef")
            .field("path", &self.path)
            .field("parent", &self.parent)
            .field("sender", &self.sender)
            .finish()
    }
}

impl TActorRef for DeferredActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, _: Option<ActorRef>) {
        let _ = self.sender.try_send(message);
    }

    fn start(&self) {}

    fn stop(&self) {}

    fn resume(&self) {}

    fn suspend(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        Some(&self.parent)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl DeferredActorRef {
    pub(crate) fn new(
        path: ActorPath,
        parent: ActorRef,
    ) -> (Self, Receiver<DynMessage>) {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let inner = DeferredActorRefInner {
            path,
            parent,
            sender: tx,
        };
        let deferred_ref = DeferredActorRef(inner.into());
        (deferred_ref, rx)
    }
    pub(crate) async fn ask(
        &self,
        target: &ActorRef,
        mut rx: Receiver<DynMessage>,
        message: DynMessage,
        timeout: Duration,
    ) -> Result<Option<DynMessage>, Elapsed> {
        target.tell(message, Some(self.clone().into()));
        let resp = tokio::time::timeout(timeout, rx.recv()).await;
        self.provider.unregister_temp_actor(&self.path);
        resp
    }

    pub(crate) async fn ask_selection(
        &self,
        actor_sel: &ActorSelection,
        mut rx: Receiver<DynMessage>,
        message: DynMessage,
        timeout: Duration,
    ) -> Result<Option<DynMessage>, Elapsed> {
        actor_sel.tell(message, Some(self.clone().into()));
        let resp = tokio::time::timeout(timeout, rx.recv()).await;
        self.provider.unregister_temp_actor(&self.path);
        resp
    }
}

impl Into<ActorRef> for DeferredActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
