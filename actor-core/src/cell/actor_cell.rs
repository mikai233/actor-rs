use crate::actor::actor_system::ActorSystem;
use crate::actor::context::{ActorContext, ActorContext1};
use crate::actor::props::Props;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::function_ref::FunctionRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::envelope::Envelope;
use crate::cell::Cell;
use crate::message::DynMessage;
use crate::provider::ActorRefProvider;
use ahash::{HashMap, RandomState};
use anyhow::Error;
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub(crate) struct ActorCell {
    system: ActorSystem,
    myself: ActorRef,
    function_refs: HashMap<String, FunctionRef>,
}

impl ActorCell {
    pub(crate) fn get_child_by_name(&self, name: &str) -> Option<ActorRef> {
        self.children_refs().get(name).map(|c| c.value().clone())
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

    pub(crate) fn add_function_ref(&mut self, name: String, function_ref: FunctionRef) {
        self.function_refs.insert(name, function_ref);
    }

    pub(crate) fn remove_function_ref(&mut self, name: &str) -> Option<FunctionRef> {
        self.function_refs.remove(name)
    }

    pub(crate) fn get_function_ref(&self, name: &str) -> Option<&FunctionRef> {
        self.function_refs.get(name)
    }

    pub(crate) fn insert_child(&self, name: String, child: impl Into<ActorRef>) {
        self.children_refs().insert(name, child.into());
    }

    pub(crate) fn remove_child(&self, name: &String) -> Option<ActorRef> {
        match self.children_refs().remove(name) {
            None => {
                None
            }
            Some((_, child)) => {
                Some(child)
            }
        }
    }

    fn handle_system_message(
        ctx: &mut ActorContext1,
        envelope: Envelope,
    ) {
        let Envelope { message, sender } = envelope;
        let name = message.signature().name;
        if let Some(error) = system_receive.receive(actor, ctx, message, sender).err() {
            ctx.handle_invoke_failure(name, error);
        }
    }
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
        self.myself.parent()
    }

    fn children_refs(&self) -> &DashMap<String, ActorRef, RandomState> {
        &self.myself.local().unwrap().children
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
        &self.system
    }

    fn provider(&self) -> &ActorRefProvider {
        &self.system.provider
    }

    fn guardian(&self) -> &LocalActorRef {
        self.system.guardian()
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

impl ActorContext for ActorCell {
    fn new(system: ActorSystem, myself: ActorRef) -> Self {
        Self {
            system,
            myself,
            function_refs: Default::default(),
        }
    }

    fn myself(&self) -> &ActorRef {
        &self.myself
    }

    fn children(&self) -> Vec<ActorRef> {
        todo!()
    }

    fn child(&self, name: &str) -> Option<ActorRef> {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.myself.parent()
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