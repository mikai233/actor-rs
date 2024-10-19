use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;

use actor_core::actor_path::ActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use actor_core::message::poison_pill::PoisonPill;
use actor_core::message::resume::Resume;
use actor_core::message::suspend::Suspend;
use actor_core::message::unwatch::Unwatch;
use actor_core::message::watch::Watch;
use actor_core::message::{downcast_into, DynMessage, Message};
use actor_core::AsAny;

use crate::artery::outbound_message::OutboundMessage;
use crate::remote_watcher::unwatch_remote::UnwatchRemote;
use crate::remote_watcher::watch_remote::WatchRemote;

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct RemoteActorRef(Arc<RemoteActorRefInner>);

pub struct RemoteActorRefInner {
    pub(crate) path: ActorPath,
    pub(crate) transport: ActorRef,
    pub(crate) remote_watcher: ActorRef,
}

impl RemoteActorRef {
    pub(crate) fn new(path: ActorPath, transport: ActorRef, remote_watcher: ActorRef) -> Self {
        let inner = RemoteActorRefInner {
            path,
            transport,
            remote_watcher,
        };
        Self(inner.into())
    }

    pub(crate) fn is_watch_intercepted(&self, watchee: &ActorRef, watcher: &ActorRef) -> bool {
        watcher != &self.remote_watcher && watchee.path() == self.path()
    }

    fn send_remote(&self, message: DynMessage, sender: Option<ActorRef>) {
        let message = OutboundMessage::new(message, sender, self.clone().into());
        self.transport.cast_ns(message);
    }
}

impl Debug for RemoteActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("RemoteActorRef")
            .field("path", &self.path)
            .field("transport", &self.transport)
            .field("remote_watcher", &self.remote_watcher)
            .finish()
    }
}

impl TActorRef for RemoteActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.signature().name;
        if name == Watch::signature_sized().name {
            let Watch { watchee, watcher } =
                *downcast_into(message).expect("message is not Watch message");
            if self.is_watch_intercepted(&watchee, &watcher) {
                self.remote_watcher
                    .cast_ns(WatchRemote { watchee, watcher });
            } else {
                let watch = Watch { watchee, watcher };
                self.send_remote(Box::new(watch), sender);
            }
        } else if name == Unwatch::signature_sized().name {
            let Unwatch { watchee, watcher } =
                *downcast_into(message).expect("message is not Unwatch message");
            if self.is_watch_intercepted(&watchee, &watcher) {
                self.remote_watcher
                    .cast_ns(UnwatchRemote { watchee, watcher });
            } else {
                let unwatch = Unwatch { watchee, watcher };
                self.send_remote(Box::new(unwatch), sender);
            }
        } else {
            self.send_remote(message, sender);
        }
    }

    fn stop(&self) {
        self.cast_ns(PoisonPill);
    }

    fn parent(&self) -> Option<&dyn TActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        match names.peek() {
            None => Some(self.clone().into()),
            Some(&"..") => None,
            Some(_) => {
                let remote_ref = RemoteActorRef::new(
                    self.path().descendant(names),
                    self.transport.clone(),
                    self.remote_watcher.clone(),
                );
                Some(remote_ref.into())
            }
        }
    }

    fn resume(&self) {
        self.cast_ns(Resume);
    }

    fn suspend(&self) {
        self.cast_ns(Suspend);
    }

    fn start(&self) {}
}

impl Into<ActorRef> for RemoteActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
