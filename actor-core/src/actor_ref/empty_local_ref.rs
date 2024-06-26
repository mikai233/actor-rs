use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::actor_system::WeakActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, ActorRefSystemExt, get_child_default, TActorRef};
use crate::DynMessage;
use crate::ext::option_ext::OptionExt;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;

#[derive(Clone, AsAny)]
pub struct EmptyLocalActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: WeakActorSystem,
    pub(crate) path: ActorPath,
}

impl Debug for EmptyLocalActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EmptyLocalActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .finish()
    }
}

impl Deref for EmptyLocalActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TActorRef for EmptyLocalActorRef {
    fn system(&self) -> &WeakActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.special_handle(message, sender)
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl EmptyLocalActorRef {
    pub(crate) fn new(system: WeakActorSystem, path: ActorPath) -> Self {
        Self {
            inner: Arc::new(Inner { system, path }),
        }
    }

    fn special_handle(&self, message: DynMessage, sender: Option<ActorRef>) {
        let watch = type_name::<Watch>();
        let unwatch = type_name::<Unwatch>();
        let identify = type_name::<Identify>();
        let actor_selection = type_name::<ActorSelectionMessage>();
        if message.name == watch {
            let watch = message.downcast_system::<Watch>().unwrap();
            if watch.watchee.path() == self.path() && watch.watcher.path() != self.path() {
                let notification = DeathWatchNotification {
                    actor: watch.watchee,
                    existence_confirmed: true,
                    address_terminated: false,
                };
                watch.watcher.cast_system(notification, ActorRef::no_sender());
            }
        } else if message.name == unwatch {
            // just ignore
        } else if message.name == identify {
            sender.foreach(|s| s.cast_orphan_ns(ActorIdentity { actor_ref: None }));
        } else if message.name == actor_selection {
            let actor_selection = message.downcast_orphan::<ActorSelectionMessage>().unwrap();
            if actor_selection.identify_request().is_some() {
                if !actor_selection.wildcard_fan_out {
                    sender.foreach(|s| s.cast_orphan_ns(ActorIdentity { actor_ref: None }));
                }
            }
        }
    }
}

impl Into<ActorRef> for EmptyLocalActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}