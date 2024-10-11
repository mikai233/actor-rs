use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};

use actor_derive::AsAny;

use crate::actor_path::ActorPath;
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

    fn parent(&self) -> Option<&dyn TActorRef> {
        Some(&self.parent)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl DeferredActorRef {
    pub(crate) fn new(path: ActorPath, parent: ActorRef) -> (Self, Receiver<DynMessage>) {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let inner = DeferredActorRefInner {
            path,
            parent,
            sender: tx,
        };
        let deferred_ref = DeferredActorRef(inner.into());
        (deferred_ref, rx)
    }
}

impl Into<ActorRef> for DeferredActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
