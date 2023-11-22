use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::error::Elapsed;

use crate::{DeferredMessage, DynamicMessage, Message};
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct DeferredActorRef {
    system: ActorSystem,
    provider: Box<ActorRefProvider>,
    path: ActorPath,
    parent: Box<ActorRef>,
    sender: Sender<DynamicMessage>,
    message_name: &'static str,
}

impl TActorRef for DeferredActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynamicMessage, _sender: Option<ActorRef>) {
        let _ = self.sender.try_send(message);
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        Some(&self.parent)
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        let mut names = names.into_iter();
        match names.next() {
            None => {
                Some(self.clone().into())
            }
            Some(_) => {
                None
            }
        }
    }
}

impl DeferredActorRef {
    pub(crate) fn new(system: ActorSystem, target_name: String, message_name: &'static str) -> (Self, Receiver<DynamicMessage>) {
        let provider = system.provider();
        let path = provider.temp_path_of_prefix(Some(target_name));
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let parent = Box::new(provider.temp_container().clone());
        let deferred_ref = DeferredActorRef {
            system,
            provider: Box::new(provider),
            path: path.clone(),
            parent,
            sender: tx,
            message_name,
        };
        deferred_ref.provider.register_temp_actor(deferred_ref.clone().into(), deferred_ref.path());
        (deferred_ref, rx)
    }
    pub(crate) async fn ask(&self, target: &ActorRef, mut rx: Receiver<DynamicMessage>, message: DynamicMessage, timeout: Duration) -> Result<Option<DynamicMessage>, Elapsed> {
        target.tell(message, Some(self.clone().into()));
        let resp = tokio::time::timeout(timeout, rx.recv()).await;
        self.provider.unregister_temp_actor(&self.path);
        resp
    }
}

pub struct Patterns;

impl Patterns {
    pub async fn ask<Req, Resp>(actor: &ActorRef, message: Req, timeout: Duration) -> anyhow::Result<Resp> where Req: Message, Resp: DeferredMessage {
        let req_name = std::any::type_name::<Req>();
        let (deferred, rx) = DeferredActorRef::new(actor.system(), actor.path().name().to_string(), req_name);
        match deferred.ask(actor, rx, DynamicMessage::user(message), timeout).await {
            Ok(Some(resp)) => {
                match resp {
                    DynamicMessage::Deferred(m) => {
                        match m.inner.into_any().downcast::<Resp>() {
                            Ok(resp) => {
                                Ok(*resp)
                            }
                            Err(_) => {
                                let resp_name = std::any::type_name::<Resp>();
                                Err(anyhow!("{} ask {} expect {} resp, but found other type resp", actor, req_name, resp_name))
                            }
                        }
                    }
                    _ => {
                        Err(anyhow!("{} ask {} expect Deferred resp, but found other type message", actor, req_name))
                    }
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