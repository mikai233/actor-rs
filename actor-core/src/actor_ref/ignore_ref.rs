use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::actor_system::WeakSystem;
use crate::actor::address::{Address, Protocol};
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_path::root_actor_path::RootActorPath;
use crate::actor_ref::{ActorRef, get_child_default, TActorRef};
use crate::DynMessage;

#[derive(Clone, AsAny)]
pub struct IgnoreActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: WeakSystem,
    pub(crate) path: ActorPath,
}

impl Debug for IgnoreActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("IgnoreActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .finish()
    }
}

impl Deref for IgnoreActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TActorRef for IgnoreActorRef {
    fn system(&self) -> &WeakSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, _message: DynMessage, _sender: Option<ActorRef>) {}

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl Into<ActorRef> for IgnoreActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl IgnoreActorRef {
    pub(crate) fn new(system: WeakSystem) -> Self {
        let path = Self::path();
        Self {
            inner: Arc::new(Inner { system, path }),
        }
    }

    fn fake_system_name() -> &'static str {
        "local"
    }

    fn path() -> ActorPath {
        RootActorPath::new(Address::new(Protocol::Akka, Self::fake_system_name(), None), "/")
            .child("ignore")
            .into()
    }

    fn is_ignore_ref_path_str(&self, other_path: &str) -> bool {
        self.path.to_string() == other_path
    }

    fn is_ignore_ref_path(&self, other_path: &ActorPath) -> bool {
        &self.path == other_path
    }
}