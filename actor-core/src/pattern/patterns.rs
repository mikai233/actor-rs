use std::future::Future;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::anyhow;
use tokio::time::error::Elapsed;

use crate::actor::actor_selection::ActorSelection;
use crate::actor_path::TActorPath;
use crate::actor_ref::deferred_ref::DeferredActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::{downcast_into, DynMessage, Message};
use crate::provider::ActorRefProvider;

#[derive(Debug)]
pub struct Patterns;

impl Patterns {
    pub async fn ask<Request, Response>(
        actor: &ActorRef,
        message: Request,
        timeout: Duration,
        provider: &ActorRefProvider,
    ) -> anyhow::Result<Response>
    where
        Request: Message,
        Response: Message,
    {
        let path = provider.temp_path_of_prefix(Some(actor.path().name()));
        let parent = provider.temp_container();
        let (deferred, mut rx) = DeferredActorRef::new(path, parent);
        let path = deferred.path().clone();
        provider.register_temp_actor(deferred.clone().into(), &path);
        actor.tell(Box::new(message), Some(deferred.into()));
        let response = tokio::time::timeout(timeout, rx.recv()).await;
        provider.unregister_temp_actor(&path);
        Self::handle_response::<Request, Response>(actor.path().name(), response, timeout)
    }

    pub async fn ask_selection<Request, Response>(
        sel: &ActorSelection,
        message: Request,
        timeout: Duration,
        provider: &ActorRefProvider,
    ) -> anyhow::Result<Response>
    where
        Request: Message,
        Response: Message,
    {
        let mut hasher = ahash::AHasher::default();
        sel.path_str().hash(&mut hasher);
        let path_hash = format!("{:x}", hasher.finish());
        let path = provider.temp_path_of_prefix(Some(&path_hash));
        let parent = provider.temp_container();
        let (deferred, mut rx) = DeferredActorRef::new(path, parent);
        let path = deferred.path().clone();
        provider.register_temp_actor(deferred.clone().into(), &path);
        sel.tell(Box::new(message), Some(deferred.into()));
        let response = tokio::time::timeout(timeout, rx.recv()).await;
        provider.unregister_temp_actor(&path);
        Self::handle_response::<Request, Response>(sel.anchor.path().name(), response, timeout)
    }
    fn handle_response<Request, Response>(
        target: &str,
        response: Result<Option<DynMessage>, Elapsed>,
        timeout: Duration,
    ) -> anyhow::Result<Response>
    where
        Request: Message,
        Response: Message,
    {
        match response {
            Ok(Some(response)) => {
                let response_signature = response.signature();
                match downcast_into::<Response>(response) {
                    Ok(resp) => Ok(*resp),
                    Err(_) => {
                        Err(anyhow!(
                            "ask `{}` with `{}` expect response `{}`, but found response `{}`",
                            target,
                            Request::signature_sized(),
                            Response::signature_sized(),
                            response_signature,
                            ))
                    }
                }
            }
            Ok(None) => {
                Err(anyhow!(
                    "ask `{}` with `{}` got empty response, because DeferredActorRef is dropped",
                    target,
                    Request::signature_sized(),
                ))
            }
            Err(_) => {
                Err(anyhow!(
                    "ask `{}` with `{}` timeout after {:?}, a typical reason is that the recipient actor didn't send a reply",
                    target,
                    Request::signature_sized(),
                    timeout
                ))
            }
        }
    }
}

pub trait PatternsExt {
    fn ask<Request, Response>(
        &self,
        message: Request,
        timeout: Duration,
        provider: &ActorRefProvider,
    ) -> impl Future<Output = anyhow::Result<Response>> + Send
    where
        Request: Message,
        Response: Message;
}

impl PatternsExt for ActorRef {
    fn ask<Request, Response>(
        &self,
        message: Request,
        timeout: Duration,
        provider: &ActorRefProvider,
    ) -> impl Future<Output = anyhow::Result<Response>> + Send
    where
        Request: Message,
        Response: Message,
    {
        Patterns::ask(self, message, timeout, provider)
    }
}

impl PatternsExt for ActorSelection {
    fn ask<Request, Response>(
        &self,
        message: Request,
        timeout: Duration,
        provider: &ActorRefProvider,
    ) -> impl Future<Output = anyhow::Result<Response>> + Send
    where
        Request: Message,
        Response: Message,
    {
        Patterns::ask_selection(self, message, timeout, provider)
    }
}
