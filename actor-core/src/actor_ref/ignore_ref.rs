use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::address::{Address, Protocol};
use crate::actor_path::root_actor_path::RootActorPath;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{get_child_default, ActorRef, TActorRef};
use crate::message::DynMessage;

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct IgnoreActorRef(Arc<IgnoreActorRefInner>);

pub struct IgnoreActorRefInner {
    pub(crate) path: ActorPath,
}

impl Debug for IgnoreActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("IgnoreActorRef")
            .field("path", &self.path)
            .finish()
    }
}

impl TActorRef for IgnoreActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, _: DynMessage, _: Option<ActorRef>) {}

    fn start(&self) {}

    fn stop(&self) {}

    fn resume(&self) {}

    fn suspend(&self) {}

    fn parent(&self) -> Option<&dyn TActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl Into<ActorRef> for IgnoreActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl IgnoreActorRef {
    pub(crate) fn new() -> Self {
        let path = Self::path();
        let inner = IgnoreActorRefInner { path };
        Self(inner.into())
    }

    fn fake_system_name() -> &'static str {
        "local"
    }

    fn path() -> ActorPath {
        RootActorPath::new(
            Address::new(Protocol::Tcp, Self::fake_system_name(), None),
            "/",
        )
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
