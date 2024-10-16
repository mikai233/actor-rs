use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::DynMessage;

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct FunctionRef(Arc<FunctionRefInner>);

pub struct FunctionRefInner {
    pub(crate) path: ActorPath,
    pub(crate) transform: Box<dyn Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static>,
}

impl Debug for FunctionRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionRef")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl TActorRef for FunctionRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        (self.transform)(message, sender);
    }

    fn start(&self) {}

    fn stop(&self) {}

    fn resume(&self) {}

    fn suspend(&self) {}

    fn parent(&self) -> Option<&dyn TActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        match names.next() {
            None => Some(self.clone().into()),
            Some(_) => None,
        }
    }
}

impl Into<ActorRef> for FunctionRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl FunctionRef {
    pub fn new<F>(path: impl Into<ActorPath>, transform: F) -> Self
    where
        F: Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static,
    {
        let inner = FunctionRefInner {
            path: path.into(),
            transform: Box::new(transform),
        };
        FunctionRef(inner.into())
    }
}
