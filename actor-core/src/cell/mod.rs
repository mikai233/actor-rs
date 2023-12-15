use std::sync::Arc;

use dashmap::DashMap;

use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::ActorRef;
use crate::actor::fault_handing::ChildRestartStats;
use crate::actor::function_ref::FunctionRef;

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
            restart_stats: Default::default(),
            function_refs: Default::default(),
        };
        Self {
            inner: inner.into(),
        }
    }

    pub(crate) fn parent(&self) -> Option<&ActorRef> {
        self.inner.parent.as_ref()
    }

    pub(crate) fn children(&self) -> &DashMap<String, ActorRef> {
        &self.inner.children
    }

    pub(crate) fn get_child_by_name(&self, name: &String) -> Option<ActorRef> {
        self.children().get(name).map(|c| c.value().clone())
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

    pub(crate) fn restart_stats(&self) -> &DashMap<ActorRef, ChildRestartStats> {
        &self.inner.restart_stats
    }

    pub(crate) fn insert_child(&self, name: String, child: impl Into<ActorRef>) {
        let child = child.into();
        self.inner.children.insert(name, child.clone());
        self.inner.restart_stats.insert(child, ChildRestartStats::default());
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<ActorRef> {
        match self.inner.children.remove(name) {
            None => {
                None
            }
            Some((_, child)) => {
                self.inner.restart_stats.remove(&child);
                Some(child)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    parent: Option<ActorRef>,
    children: DashMap<String, ActorRef>,
    restart_stats: DashMap<ActorRef, ChildRestartStats>,
    function_refs: DashMap<String, FunctionRef>,
}
