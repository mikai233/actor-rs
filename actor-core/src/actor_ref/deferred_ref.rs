use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::Guard;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::error::Elapsed;

use actor_derive::AsAny;

use crate::actor::actor_selection::ActorSelection;
use crate::actor::actor_system::WeakActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, get_child_default, TActorRef};
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::DynMessage;
use crate::provider::ActorRefProvider;

#[derive(Clone, AsAny)]
pub struct DeferredActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    system: WeakActorSystem,
    provider: Guard<Arc<ActorRefProvider>>,
    path: ActorPath,
    parent: ActorRef,
    sender: Sender<DynMessage>,
    message_name: &'static str,
}

impl Deref for DeferredActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for DeferredActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredActorRef")
            .field("system", &"..")
            .field("provider", &self.provider)
            .field("path", &self.path)
            .field("parent", &self.parent)
            .field("sender", &self.sender)
            .field("message_name", &self.message_name)
            .finish()
    }
}

impl TActorRef for DeferredActorRef {
    fn system(&self) -> &WeakActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, _sender: Option<ActorRef>) {
        let _ = self.sender.try_send(message);
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        Some(&self.parent)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl DeferredActorRef {
    pub(crate) fn new(system: WeakActorSystem, ref_path_prefix: &String, message_name: &'static str) -> anyhow::Result<(Self, Receiver<DynMessage>)> {
        let provider = system.upgrade()?.provider();
        let path = provider.temp_path_of_prefix(Some(ref_path_prefix));
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let parent = provider.temp_container();
        let inner = Inner {
            system,
            provider,
            path,
            parent,
            sender: tx,
            message_name,
        };
        let deferred_ref = DeferredActorRef {
            inner: inner.into(),
        };
        deferred_ref.provider.register_temp_actor(deferred_ref.clone().into(), deferred_ref.path());
        Ok((deferred_ref, rx))
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