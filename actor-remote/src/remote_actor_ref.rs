use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use tracing::error;

use actor_core::actor::actor_system::WeakActorSystem;
use actor_core::actor_path::ActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt, TActorRef};
use actor_core::DynMessage;
use actor_core::ext::message_ext::SystemMessageExt;
use actor_core::message::message_registration::MessageRegistration;
use actor_core::message::poison_pill::PoisonPill;
use actor_core::message::resume::Resume;
use actor_core::message::suspend::Suspend;
use actor_core::message::unwatch::Unwatch;
use actor_core::message::watch::Watch;
use actor_derive::AsAny;

use crate::remote_watcher::unwatch_remote::UnwatchRemote;
use crate::remote_watcher::watch_remote::WatchRemote;
use crate::transport::outbound_message::OutboundMessage;
use crate::transport::remote_envelope::RemoteEnvelope;

#[derive(Clone, AsAny)]
pub struct RemoteActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: WeakActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) transport: ActorRef,
    pub(crate) registration: Arc<MessageRegistration>,
    pub(crate) remote_watcher: ActorRef,
}

impl RemoteActorRef {
    pub(crate) fn new(
        system: WeakActorSystem,
        path: ActorPath,
        transport: ActorRef,
        registration: Arc<MessageRegistration>,
        remote_watcher: ActorRef,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                system,
                path,
                transport,
                registration,
                remote_watcher,
            }),
        }
    }

    pub(crate) fn is_watch_intercepted(&self, watchee: &ActorRef, watcher: &ActorRef) -> bool {
        watcher != &self.remote_watcher && watchee.path() == self.path()
    }

    fn send_remote(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.name();
        match self.registration.encode_boxed(&message) {
            Ok(packet) => {
                let envelope = RemoteEnvelope {
                    packet,
                    sender,
                    target: self.clone().into(),
                };
                self.transport.cast_ns(OutboundMessage { name, envelope });
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
    fn system(&self) -> &WeakActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.name();
        if name == type_name::<Watch>() {
            let Watch { watchee, watcher } = message.downcast_system::<Watch>().unwrap();
            if self.is_watch_intercepted(&watchee, &watcher) {
                self.remote_watcher.cast_ns(WatchRemote { watchee, watcher });
            } else {
                let watch = Watch { watchee, watcher }.into_dyn();
                self.send_remote(watch, sender);
            }
        } else if name == type_name::<Unwatch>() {
            let Unwatch { watchee, watcher } = message.downcast_system::<Unwatch>().unwrap();
            if self.is_watch_intercepted(&watchee, &watcher) {
                self.remote_watcher.cast_ns(UnwatchRemote { watchee, watcher });
            } else {
                let unwatch = Unwatch { watchee, watcher }.into_dyn();
                self.send_remote(unwatch, sender);
            }
        } else {
            self.send_remote(message, sender);
        }
    }

    fn stop(&self) {
        self.cast_system(PoisonPill, None);
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        match names.peek() {
            None => {
                Some(self.clone().into())
            }
            Some(&"..") => None,
            Some(_) => {
                let remote_ref = RemoteActorRef::new(
                    self.system.clone(),
                    self.path().descendant(names),
                    self.transport.clone(),
                    self.registration.clone(),
                    self.remote_watcher.clone(),
                );
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
}

impl Into<ActorRef> for RemoteActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}