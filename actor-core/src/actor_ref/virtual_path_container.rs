use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Not;
use std::sync::Arc;

use dashmap::mapref::one::Ref;
use dashmap::DashMap;

use actor_derive::AsAny;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::terminate::Terminate;
use crate::message::{DynMessage, Message};

#[derive(Clone, derive_more::Deref, AsAny)]
pub struct VirtualPathContainer(Arc<VirtualPathContainerInner>);

pub struct VirtualPathContainerInner {
    pub(crate) path: ActorPath,
    pub(crate) parent: ActorRef,
    pub(crate) children: DashMap<String, ActorRef, ahash::RandomState>,
}

impl Debug for VirtualPathContainer {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("VirtualPathContainer")
            .field("path", &self.path)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .finish()
    }
}

impl TActorRef for VirtualPathContainer {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, _sender: Option<ActorRef>) {
        if message.signature() == Terminate::signature_sized() {
            let notification = DeathWatchNotification {
                actor: self.clone().into(),
                existence_confirmed: true,
                address_terminated: false,
            };
            self.parent.cast_ns(notification);
        }
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&dyn TActorRef> {
        Some(&self.parent)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        match names.next() {
            None => Some(self.clone().into()),
            Some(name) => {
                if name.is_empty() {
                    Some(self.clone().into())
                } else {
                    match self.children.get(name) {
                        None => None,
                        Some(child) => child.value().get_child(names),
                    }
                }
            }
        }
    }

    fn start(&self) {}

    fn resume(&self) {}

    fn suspend(&self) {}
}

impl Into<ActorRef> for VirtualPathContainer {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl VirtualPathContainer {
    pub(crate) fn new(path: ActorPath, parent: ActorRef) -> Self {
        let inner = VirtualPathContainerInner {
            path,
            parent,
            children: DashMap::with_hasher(ahash::RandomState::default()),
        };
        Self(inner.into())
    }

    pub(crate) fn add_child(&self, name: String, child: ActorRef) {
        if let Some(old) = self.children.insert(name, child) {
            old.stop();
        }
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<(String, ActorRef)> {
        self.children.remove(name)
    }

    pub(crate) fn remove_child_ref(
        &self,
        name: &String,
        child: &ActorRef,
    ) -> Option<(String, ActorRef)> {
        self.children.remove_if(name, |_, c| c == child)
    }

    pub(crate) fn get_child(
        &self,
        name: &String,
    ) -> Option<Ref<String, ActorRef, ahash::RandomState>> {
        self.children.get(name)
    }

    pub(crate) fn has_children(&self) -> bool {
        self.children.is_empty().not()
    }

    pub(crate) fn foreach_child(&self, f: impl Fn(&String, &ActorRef)) {
        for child in self.children.iter() {
            f(child.key(), child.value());
        }
    }
}
