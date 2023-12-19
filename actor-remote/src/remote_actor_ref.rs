use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use tracing::error;

use actor_core::actor::actor_path::ActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt, TActorRef};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::DynMessage;
use actor_core::message::poison_pill::PoisonPill;
use actor_core::message::recreate::Recreate;
use actor_core::message::resume::Resume;
use actor_core::message::suspend::Suspend;
use actor_derive::AsAny;

use crate::message_registration::MessageRegistration;
use crate::net::message::{OutboundMessage, RemoteEnvelope};

#[derive(Clone, AsAny)]
pub struct RemoteActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) transport: Arc<ActorRef>,
    pub(crate) registration: Arc<MessageRegistration>,
}

impl Deref for RemoteActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for RemoteActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("RemoteActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .field("transport", &self.transport)
            .finish()
    }
}

impl TActorRef for RemoteActorRef {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.name();
        match self.registration.encode_boxed(message) {
            Ok(packet) => {
                let envelope = RemoteEnvelope {
                    packet,
                    sender,
                    target: self.clone().into(),
                };
                self.transport.cast(OutboundMessage { name, envelope }, None);
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

    fn get_child(&self, names: Box<dyn Iterator<Item=String>>) -> Option<ActorRef> {
        let mut names = names.into_iter().peekable();
        match names.peek().map(|n| n.as_str()) {
            None => {
                Some(self.clone().into())
            }
            Some("..") => None,
            Some(_) => {
                let inner = Inner {
                    system: self.system().clone(),
                    path: self.path().descendant(names),
                    transport: self.transport.clone(),
                    registration: self.registration.clone(),
                };
                let remote_ref = RemoteActorRef {
                    inner: inner.into(),
                };
                Some(remote_ref.into())
            }
        }
    }

    fn resume(&self) {
        self.cast_system(Resume, ActorRef::no_sender());
    }

    fn suspend(&self) {
        self.cast_system(Suspend, ActorRef::no_sender());
    }

    fn restart(&self) {
        self.cast_system(Recreate, ActorRef::no_sender());
    }
}

impl Into<ActorRef> for RemoteActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}