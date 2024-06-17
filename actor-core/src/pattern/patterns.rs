use std::any::type_name;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::anyhow;
use tokio::time::error::Elapsed;

use crate::{CodecMessage, DynMessage, MessageType, OrphanMessage, SystemMessage};
use crate::actor::actor_selection::ActorSelection;
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::deferred_ref::DeferredActorRef;

#[derive(Debug)]
pub struct Patterns;

impl Patterns {
    pub async fn ask<Req, Resp>(
        actor: &ActorRef,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Resp>
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        let message = message.into_dyn();
        Self::internal_ask::<Req, Resp>(actor, timeout, message).await
    }

    pub async fn ask_sys<Req, Resp>(
        actor: &ActorRef,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Resp>
        where
            Req: SystemMessage,
            Resp: OrphanMessage {
        let message = DynMessage::system(message);
        Self::internal_ask::<Req, Resp>(actor, timeout, message).await
    }

    async fn internal_ask<Req, Resp>(
        actor: &ActorRef,
        timeout: Duration,
        message: DynMessage,
    ) -> anyhow::Result<Resp>
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        let req = type_name::<Req>();
        let (deferred, rx) = DeferredActorRef::new(
            actor.system().clone(),
            actor.path().name(),
            req,
        )?;
        let resp = deferred.ask(actor, rx, message, timeout).await;
        Self::handle_resp::<Req, Resp>(actor.to_string(), resp, timeout)
    }

    pub async fn ask_selection<Req, Resp>(
        sel: &ActorSelection,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Resp>
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        let message = message.into_dyn();
        Self::internal_ask_selection::<Req, Resp>(sel, timeout, message).await
    }

    pub async fn ask_selection_sys<Req, Resp>(
        sel: &ActorSelection,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Resp>
        where
            Req: SystemMessage,
            Resp: OrphanMessage {
        let message = DynMessage::system(message);
        Self::internal_ask_selection::<Req, Resp>(sel, timeout, message).await
    }

    async fn internal_ask_selection<Req, Resp>(
        sel: &ActorSelection,
        timeout: Duration,
        message: DynMessage,
    ) -> anyhow::Result<Resp>
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        let req_name = type_name::<Req>();
        let mut hasher = ahash::AHasher::default();
        sel.path_str().hash(&mut hasher);
        let path_hash = hasher.finish();
        let (deferred, rx) = DeferredActorRef::new(
            sel.anchor.system().clone(),
            &format!("{}", path_hash),
            req_name,
        )?;
        let resp = deferred.ask_selection(sel, rx, message, timeout).await;
        Self::handle_resp::<Req, Resp>(sel.anchor.to_string(), resp, timeout)
    }

    fn handle_resp<Req, Resp>(
        target: String,
        resp: Result<Option<DynMessage>, Elapsed>,
        timeout: Duration,
    ) -> anyhow::Result<Resp>
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        match resp {
            Ok(Some(resp)) => {
                let message = resp.message.into_any();
                let message_type = resp.ty;
                if matches!(message_type, MessageType::Orphan) {
                    match message.downcast::<Resp>() {
                        Ok(resp) => {
                            Ok(*resp)
                        }
                        Err(_) => {
                            let req = type_name::<Req>();
                            let resp = type_name::<Resp>();
                            Err(anyhow!("ask {} with {} expect {} resp, but found other resp", target, req, resp))
                        }
                    }
                } else {
                    let req = type_name::<Req>();
                    Err(anyhow!("ask {} with {} expect OrphanMessage resp, but found other type message", target, req))
                }
            }
            Ok(None) => {
                let req = type_name::<Req>();
                Err(anyhow!("ask {} with {} got empty resp, because DeferredActorRef is dropped", target, req))
            }
            Err(_) => {
                let req = type_name::<Req>();
                Err(anyhow!("ask {} with {} timeout after {:?}, a typical reason is that the recipient actor didn't send a reply", target, req, timeout))
            }
        }
    }
}

pub trait PatternsExt {
    fn ask<Req, Resp>(
        &self,
        message: Req,
        timeout: Duration,
    ) -> impl Future<Output=anyhow::Result<Resp>> + Send
        where
            Req: CodecMessage,
            Resp: OrphanMessage;
}

impl PatternsExt for ActorRef {
    fn ask<Req, Resp>(
        &self,
        message: Req,
        timeout: Duration,
    ) -> impl Future<Output=anyhow::Result<Resp>> + Send
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        Patterns::ask(self, message, timeout)
    }
}

impl PatternsExt for ActorSelection {
    fn ask<Req, Resp>(
        &self,
        message: Req,
        timeout: Duration,
    ) -> impl Future<Output=anyhow::Result<Resp>> + Send
        where
            Req: CodecMessage,
            Resp: OrphanMessage {
        Patterns::ask_selection(self, message, timeout)
    }
}