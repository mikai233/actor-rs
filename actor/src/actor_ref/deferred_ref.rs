use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::error::Elapsed;

use crate::actor::{DeferredMessage, DynamicMessage, Message};
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct DeferredActorRef {
    system: ActorSystem,
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
        None
    }
}

impl DeferredActorRef {
    pub(crate) fn new(system: ActorSystem, target_name: String, message_name: &'static str) -> (Self, Receiver<DynamicMessage>) {
        let provider = system.provider();
        let path = provider.temp_path_of_prefix(Some(target_name));
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let deferred_ref = DeferredActorRef {
            system,
            path: path.clone(),
            parent: Box::new(provider.temp_container().clone()),
            sender: tx,
            message_name,
        };
        provider.register_temp_actor(deferred_ref.clone().into(), deferred_ref.path());
        (deferred_ref, rx)
    }
    pub(crate) async fn ask(&self, target: ActorRef, mut rx: Receiver<DynamicMessage>, message: DynamicMessage, timeout: Duration) -> Result<Option<DynamicMessage>, Elapsed> {
        target.tell(message, Some(self.clone().into()));
        tokio::time::timeout(timeout, rx.recv()).await
    }
}

pub struct ActorAsk;

impl ActorAsk {
    pub async fn ask<Req, Resp>(actor: ActorRef, message: Req, timeout: Duration) -> anyhow::Result<Resp> where Req: Message, Resp: DeferredMessage {
        let message_name = std::any::type_name::<Req>();
        let (deferred, rx) = DeferredActorRef::new(actor.system(), actor.path().name().to_string(), message_name);
        match deferred.ask(actor, rx, DynamicMessage::user(message), timeout).await {
            Ok(Some(resp)) => {}
            Ok(None) => {}
            Err(error) => {}
        }
        todo!()
    }
}