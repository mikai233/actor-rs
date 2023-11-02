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
    pub(crate) fn new(parent: Option<ActorRef>) -> Self {
        let inner = Inner {
            parent,
            children: Default::default(),
        };
        Self {
            inner: inner.into(),
        }
    }
    pub(crate) fn parent(&self) -> Option<&ActorRef> {
        self.inner.parent.as_ref()
    }
    pub(crate) fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>> {
        &self.inner.children
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    parent: Option<ActorRef>,
    children: RwLock<BTreeMap<String, ActorRef>>,
}