use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::error::Elapsed;

use actor_derive::AsAny;

use crate::{DynMessage, Message, MessageType, UntypedMessage};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::{ActorRef, get_child_default, TActorRef};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::actor_system::ActorSystem;

#[derive(Clone, AsAny)]
pub struct DeferredActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    system: ActorSystem,
    provider: Arc<ActorRefProvider>,
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
    fn system(&self) -> &ActorSystem {
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

    fn get_child(&self, names: Box<dyn Iterator<Item=String>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl DeferredActorRef {
    pub(crate) fn new(system: ActorSystem, target_name: String, message_name: &'static str) -> (Self, Receiver<DynMessage>) {
        let provider = system.provider().clone();
        let path = provider.temp_path_of_prefix(Some(target_name));
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let parent = provider.temp_container().clone();
        let inner = Inner {
            system,
            provider,
            path: path.clone(),
            parent,
            sender: tx,
            message_name,
        };
        let deferred_ref = DeferredActorRef {
            inner: inner.into(),
        };
        deferred_ref.provider.register_temp_actor(deferred_ref.clone().into(), deferred_ref.path());
        (deferred_ref, rx)
    }
    pub(crate) async fn ask(&self, target: &ActorRef, mut rx: Receiver<DynMessage>, message: DynMessage, timeout: Duration) -> Result<Option<DynMessage>, Elapsed> {
        target.tell(message, Some(self.clone().into()));
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

pub struct Patterns;

impl Patterns {
    pub async fn ask<Req, Resp>(actor: &ActorRef, message: Req, timeout: Duration) -> anyhow::Result<Resp> where Req: Message, Resp: UntypedMessage {
        let req_name = std::any::type_name::<Req>();
        let (deferred, rx) = DeferredActorRef::new(actor.system().clone(), actor.path().name().to_string(), req_name);
        match deferred.ask(actor, rx, DynMessage::user(message), timeout).await {
            Ok(Some(resp)) => {
                let message = resp.boxed.into_any();
                let message_type = resp.message_type;
                if matches!(message_type, MessageType::Untyped) {
                    match message.downcast::<Resp>() {
                        Ok(resp) => {
                            Ok(*resp)
                        }
                        Err(_) => {
                            let resp_name = std::any::type_name::<Resp>();
                            Err(anyhow!("{} ask {} expect {} resp, but found other type resp", actor, req_name, resp_name))
                        }
                    }
                } else {
                    Err(anyhow!("{} ask {} expect Deferred resp, but found other type message", actor, req_name))
                }
            }
            Ok(None) => {
                Err(anyhow!("{} ask {} got empty resp, because DeferredActorRef is dropped", actor, req_name))
            }
            Err(_) => {
                Err(anyhow!("{} ask {} timeout after {:?}, a typical reason is that the recipient actor didn't send a reply", actor, req_name, timeout))
            }
        }
    }
}