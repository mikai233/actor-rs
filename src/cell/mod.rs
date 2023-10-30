use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::actor_ref::ActorRef;

pub(crate) mod runtime;
pub(crate) mod envelope;

#[derive(Debug, Clone)]
pub(crate) struct ActorCell {
    pub(crate) inner: Arc<Inner>,
}

impl ActorCell {
    pub(crate) fn new(parent: Option<ActorCell>, myself: ActorRef) -> Self {
        let inner = Inner {
            parent,
            myself,
            children: Default::default(),
        };
        Self {
            inner: inner.into(),
        }
    }
    pub(crate) fn parent(&self) -> Option<&ActorCell> {
        self.inner.parent.as_ref()
    }
    pub(crate) fn myself(&self) -> &ActorRef {
        &self.inner.myself
    }
    pub(crate) fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>> {
        &self.inner.children
    }
}

#[derive(Debug)]
struct Inner {
    parent: Option<ActorCell>,
    myself: ActorRef,
    children: RwLock<BTreeMap<String, ActorRef>>,
}