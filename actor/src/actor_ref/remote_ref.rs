use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use tracing::error;

use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt, TActorRef};
use crate::DynamicMessage;
use crate::message::poison_pill::PoisonPill;
use crate::net::message::{OutboundMessage, RemoteEnvelope};
use crate::system::ActorSystem;

#[derive(Clone)]
pub struct RemoteActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) transport: Arc<ActorRef>,
}

impl Deref for RemoteActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for RemoteActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .field("transport", &self.transport)
            .finish()
    }
}

impl TActorRef for RemoteActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynamicMessage, sender: Option<ActorRef>) {
        let reg = self.system.registration();
        let name = message.name();
        let boxed_message = match message {
            DynamicMessage::User(message) => message,
            DynamicMessage::AsyncUser(message) => message,
            DynamicMessage::System(message) => message,
            DynamicMessage::Untyped(message) => message,
        };
        match reg.encode_boxed(boxed_message) {
            Ok(packet) => {
                let envelope = RemoteEnvelope {
                    packet,
                    sender,
                    target: self.clone().into(),
                };
                self.transport.cast(OutboundMessage { envelope }, None);
            }
            Err(err) => {
                match sender {
                    None => {
                        let target: ActorRef = self.clone().into();
                        error!("send message {} to {} error {:?}", name, target, err);
                    }
                    Some(sender) => {
                        let target: ActorRef = self.clone().into();
                        error!("send message {} from {} to {} error {:?}", name, sender, target, err);
                    }
                }
            }
        }
    }

    fn stop(&self) {
        self.cast_system(PoisonPill, None);
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        let mut names = names.into_iter().peekable();
        match names.peek().map(|n| n.as_str()) {
            None => {
                Some(self.clone().into())
            }
            Some("..") => None,
            Some(_) => {
                let inner = Inner {
                    system: self.system(),
                    path: self.path().descendant(names),
                    transport: self.transport.clone(),
                };
                let remote_ref = RemoteActorRef {
                    inner: inner.into(),
                };
                Some(remote_ref.into())
            }
        }
    }
}
