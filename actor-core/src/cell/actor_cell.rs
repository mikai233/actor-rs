use std::ops::Deref;
use std::sync::Arc;

use dashmap::DashMap;
use tokio_util::sync::CancellationToken;

use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::function_ref::FunctionRef;

#[derive(Debug, Clone)]
pub(crate) struct ActorCell {
    pub(crate) inner: Arc<Inner>,
}

impl ActorCell {
    pub(crate) fn new(parent: Option<ActorRef>) -> Self {
        let inner = Inner {
            parent,
            children: DashMap::with_hasher(ahash::RandomState::new()),
            function_refs: DashMap::with_hasher(ahash::RandomState::new()),
            token: CancellationToken::new(),
        };
        Self {
            inner: inner.into(),
        }
    }

    pub(crate) fn parent(&self) -> Option<&ActorRef> {
        self.parent.as_ref()
    }

    pub(crate) fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState> {
        &self.children
    }

    pub(crate) fn get_child_by_name(&self, name: &str) -> Option<ActorRef> {
        self.children().get(name).map(|c| c.value().clone())
    }

    pub(crate) fn get_single_child(&self, name: &str) -> Option<ActorRef> {
        match name.find('#') {
            Some(_) => {
                let (child_name, uid) = ActorPath::split_name_and_uid(name);
                match self.get_child_by_name(&child_name) {
                    Some(a) => {
                        if a.path().uid() == uid {
                            Some(a)
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            None => self.get_child_by_name(name),
        }
    }

    pub(crate) fn add_function_ref(&self, name: String, function_ref: FunctionRef) {
        self.function_refs.insert(name, function_ref);
    }

    pub(crate) fn remove_function_ref(&self, name: &str) -> Option<(String, FunctionRef)> {
        self.function_refs.remove(name)
    }

    pub(crate) fn get_function_ref(&self, name: &str) -> Option<FunctionRef> {
        self.function_refs.get(name).map(|v| v.value().clone())
    }

    pub(crate) fn insert_child(&self, name: String, child: impl Into<ActorRef>) {
        let child = child.into();
        self.children.insert(name, child.clone());
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<ActorRef> {
        match self.children.remove(name) {
            None => {
                None
            }
            Some((_, child)) => {
                Some(child)
            }
        }
    }
}

impl Deref for ActorCell {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    parent: Option<ActorRef>,
    children: DashMap<String, ActorRef, ahash::RandomState>,
    function_refs: DashMap<String, FunctionRef, ahash::RandomState>,
    pub(crate) token: CancellationToken,
}