use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::actor_system::WeakSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{get_child_default, ActorRef, ActorRefExt, TActorRef};
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
use crate::message::{downcast_into, DynMessage, Message};

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct EmptyLocalActorRef(Arc<EmptyLocalActorRefInner>);

pub struct EmptyLocalActorRefInner {
    pub(crate) path: ActorPath,
}

impl Debug for EmptyLocalActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EmptyLocalActorRef")
            .field("path", &self.path)
            .finish()
    }
}

impl TActorRef for EmptyLocalActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.special_handle(message, sender)
    }

    fn start(&self) {
        todo!()
    }

    fn stop(&self) {}

    fn resume(&self) {
        todo!()
    }

    fn suspend(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl EmptyLocalActorRef {
    pub(crate) fn new(system: WeakSystem, path: ActorPath) -> Self {
        let inner = EmptyLocalActorRefInner { path };
        Self(inner.into())
    }

    fn special_handle(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.signature().name;
        if name == Watch::signature_sized().name {
            let watch = downcast_into::<Watch>(message).unwrap();
            if watch.watchee.path() == self.path() && watch.watcher.path() != self.path() {
                let notification = DeathWatchNotification {
                    actor: watch.watchee,
                    existence_confirmed: true,
                    address_terminated: false,
                };
                watch.watcher.cast_ns(notification);
            }
        } else if name == Unwatch::signature_sized().name {
            // just ignore
        } else if name == Identify::signature_sized().name {
            if let Some(sender) = sender {
                sender.cast_ns(ActorIdentity { actor_ref: None });
            }
        } else if name == ActorSelectionMessage::signature_sized().name {
            let actor_selection = downcast_into::<ActorSelectionMessage>(message).unwrap();
            if actor_selection.identify_request().is_some() {
                if !actor_selection.wildcard_fan_out && sender.is_some() {
                    sender.unwrap().cast_ns(ActorIdentity { actor_ref: None });
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