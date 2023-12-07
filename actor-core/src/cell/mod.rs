use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use dashmap::DashMap;

use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::ActorRef;
use crate::actor_ref::function_ref::FunctionRef;

pub(crate) mod envelope;
pub(crate) mod runtime;

#[derive(Debug, Clone)]
pub(crate) struct ActorCell {
    pub(crate) inner: Arc<Inner>,
}

impl ActorCell {
    pub(crate) fn new(parent: Option<ActorRef>) -> Self {
        let inner = Inner {
            parent,
            children: Default::default(),
            function_refs: Default::default(),
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
    pub(crate) fn get_child_by_name(&self, name: &String) -> Option<ActorRef> {
        self.children().read().unwrap().get(name).cloned()
    }
    pub(crate) fn get_single_child(&self, name: &String) -> Option<ActorRef> {
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
        self.inner.function_refs.insert(name, function_ref);
    }
    pub(crate) fn remove_function_ref(&self, name: &str) -> Option<(String, FunctionRef)> {
        self.inner.function_refs.remove(name)
    }
    pub(crate) fn get_function_ref(&self, name: &str) -> Option<FunctionRef> {
        self.inner.function_refs.get(name).map(|v| v.value().clone())
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    parent: Option<ActorRef>,
    children: RwLock<BTreeMap<String, ActorRef>>,
    function_refs: DashMap<String, FunctionRef>,
}
