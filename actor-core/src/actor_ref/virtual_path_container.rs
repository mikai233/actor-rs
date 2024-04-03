use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::{Deref, Not};
use std::sync::Arc;

use dashmap::DashMap;
use dashmap::mapref::one::Ref;

use actor_derive::AsAny;

use crate::{DynMessage, MessageType};
use crate::actor::actor_system::WeakActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, ActorRefSystemExt, TActorRef};
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::terminate::Terminate;

#[derive(Clone, AsAny)]
pub struct VirtualPathContainer {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: WeakActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) parent: ActorRef,
    pub(crate) children: Arc<DashMap<String, ActorRef, ahash::RandomState>>,
}

impl Deref for VirtualPathContainer {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for VirtualPathContainer {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("VirtualPathContainer")
            .field("system", &"..")
            .field("path", &self.path)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .finish()
    }
}

impl TActorRef for VirtualPathContainer {
    fn system(&self) -> &WeakActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, _sender: Option<ActorRef>) {
        if matches!(message.ty, MessageType::System) {
            if message.name == type_name::<Terminate>() {
                let notification = DeathWatchNotification {
                    actor: self.clone().into(),
                    existence_confirmed: true,
                    address_terminated: false,
                };
                self.parent.cast_system(notification, ActorRef::no_sender());
            }
        }
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        Some(&self.parent)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        match names.next() {
            None => {
                Some(self.clone().into())
            }
            Some(name) => {
                if name.is_empty() {
                    Some(self.clone().into())
                } else {
                    match self.children.get(name) {
                        None => {
                            None
                        }
                        Some(child) => {
                            child.value().get_child(names)
                        }
                    }
                }
            }
        }
    }
}

impl Into<ActorRef> for VirtualPathContainer {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl VirtualPathContainer {
    pub(crate) fn new(system: WeakActorSystem, path: ActorPath, parent: ActorRef) -> Self {
        Self {
            inner: Arc::new(Inner {
                system,
                path,
                parent,
                children: Arc::new(Default::default()),
            }),
        }
    }

    pub(crate) fn add_child(&self, name: String, child: ActorRef) {
        if let Some(old) = self.children.insert(name, child) {
            old.stop();
        }
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<(String, ActorRef)> {
        self.children.remove(name)
    }

    pub(crate) fn remove_child_ref(&self, name: &String, child: &ActorRef) -> Option<(String, ActorRef)> {
        self.children.remove_if(name, |_, c| { c == child })
    }

    pub(crate) fn get_child(&self, name: &String) -> Option<Ref<String, ActorRef, ahash::RandomState>> {
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