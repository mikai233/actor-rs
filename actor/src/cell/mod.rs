use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;

use crate::actor::Actor;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::{check_name, random_actor_name};
use crate::props::Props;

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
}

#[derive(Debug)]
pub(crate) struct Inner {
    parent: Option<ActorRef>,
    children: RwLock<BTreeMap<String, ActorRef>>,
}
