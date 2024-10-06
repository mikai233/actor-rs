use crate::actor::actor_system::ActorSystem;
use crate::actor::context::Context;
use crate::actor::props::Props;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::function_ref::FunctionRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::ActorRef;
use crate::cell::Cell;
use crate::message::DynMessage;
use crate::provider::ActorRefProvider;
use ahash::RandomState;
use anyhow::Error;
use arc_swap::Guard;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, derive_more::Deref)]
pub(crate) struct ActorCell(Arc<ActorCellInner>);

impl ActorCell {
    pub(crate) fn new(parent: Option<ActorRef>) -> Self {
        let inner = ActorCellInner {
            parent,
            children: DashMap::with_hasher(ahash::RandomState::new()),
            function_refs: DashMap::with_hasher(ahash::RandomState::new()),
        };
        Self(inner.into())
    }

    pub(crate) fn parent(&self) -> Option<&ActorRef> {
        self.0.parent.as_ref()
    }

    pub(crate) fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState> {
        &self.inner.children
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
        self.inner.function_refs.insert(name, function_ref);
    }

    pub(crate) fn remove_function_ref(&self, name: &str) -> Option<(String, FunctionRef)> {
        self.inner.function_refs.remove(name)
    }

    pub(crate) fn get_function_ref(&self, name: &str) -> Option<FunctionRef> {
        self.inner.function_refs.get(name).map(|v| v.value().clone())
    }

    pub(crate) fn insert_child(&self, name: String, child: impl Into<ActorRef>) {
        let child = child.into();
        self.inner.children.insert(name, child.clone());
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<ActorRef> {
        match self.inner.children.remove(name) {
            None => {
                None
            }
            Some((_, child)) => {
                Some(child)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct ActorCellInner {
    system: ActorSystem,
    myself: ActorRef,
    parent: Option<ActorRef>,
    children: DashMap<String, ActorRef, RandomState>,
    function_refs: DashMap<String, FunctionRef, RandomState>,
}

impl Cell for ActorCell {
    fn myself(&self) -> &ActorRef {
        &self.myself
    }

    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn start(&self) {
        todo!()
    }

    fn suspend(&self) {
        todo!()
    }

    fn resume(&self, error: Error) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.parent.as_ref()
    }

    fn children(&self) -> &DashMap<String, ActorRef, RandomState> {
        &self.children
    }

    fn get_child_by_name(&self, name: &str) -> Option<&ActorRef> {
        todo!()
    }

    fn get_single_child(&self, name: &str) -> Option<ActorRef> {
        todo!()
    }

    fn send_message(&self, message: DynMessage, sender: Option<ActorRef>) {
        todo!()
    }

    fn send_system_message(&self, message: DynMessage, sender: Option<ActorRef>) {
        todo!()
    }
}

impl ActorRefFactory for ActorCell {
    fn system(&self) -> &ActorSystem {
        todo!()
    }

    fn provider_full(&self) -> Arc<ActorRefProvider> {
        todo!()
    }

    fn provider(&self) -> Guard<Arc<ActorRefProvider>> {
        todo!()
    }

    fn guardian(&self) -> LocalActorRef {
        todo!()
    }

    fn lookup_root(&self) -> ActorRef {
        todo!()
    }

    fn spawn(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        todo!()
    }

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef> {
        todo!()
    }

    fn stop(&self, actor: &ActorRef) {
        todo!()
    }
}

impl Context for ActorCell {
    fn myself(&self) -> &ActorRef {
        todo!()
    }

    fn children(&self) -> Vec<ActorRef> {
        todo!()
    }

    fn child(&self, name: &str) -> Option<ActorRef> {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        todo!()
    }

    fn watch(&mut self, subject: &ActorRef) -> anyhow::Result<()> {
        todo!()
    }

    fn watch_with(&mut self, subject: &ActorRef, msg: DynMessage) -> anyhow::Result<()> {
        todo!()
    }

    fn unwatch(&mut self, subject: &ActorRef) {
        todo!()
    }

    fn is_watching(&self, subject: &ActorRef) -> bool {
        todo!()
    }
}