use ahash::RandomState;
use anyhow::anyhow;
use dashmap::DashMap;
use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor::is_system_message;
use crate::actor::mailbox::MailboxSender;
use crate::actor::props::Props;
use crate::actor_path::child_actor_path::ChildActorPath;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::function_ref::FunctionRef;
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::cell::envelope::Envelope;
use crate::config::duration::Duration;
use crate::ext::{check_name, random_actor_name, random_name};
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::message::DynMessage;
use crate::provider::provider::ActorSpawn;

pub type SignalSender = tokio::sync::mpsc::Sender<()>;

pub type SignalReceiver = tokio::sync::mpsc::Receiver<()>;

#[macro_export]
macro_rules! local {
    ($actor_ref: expr) => {
        $actor_ref.local().expect("ActorRef is not LocalActorRef")
    };
}

#[derive(Clone, derive_more::Deref, AsAny)]
pub struct LocalActorRef(Arc<LocalActorRefInner>);

pub struct LocalActorRefInner {
    pub(crate) path: ActorPath,
    pub(crate) sender: MailboxSender,
    pub(crate) parent: Option<ActorRef>,
    pub(crate) children: DashMap<String, ActorRef, RandomState>,
    pub(crate) function_refs: DashMap<String, FunctionRef, RandomState>,
    pub(crate) signal: SignalSender,
}

impl Debug for LocalActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalActorRef")
            .field("path", &self.path)
            .field("sender", &self.sender)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("function_refs", &self.function_refs)
            .field("signal", &self.signal)
            .finish()
    }
}

impl TActorRef for LocalActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let envelope = Envelope { message, sender };
        if is_system_message(&envelope.message) {
            self.handle_result(self.sender.system.try_send(envelope));
        } else {
            self.handle_result(self.sender.message.try_send(envelope));
        }
    }

    fn start(&self) {
        let _ = self.signal.try_send(());
    }

    fn stop(&self) {
        self.cast_ns(PoisonPill);
    }

    fn resume(&self) {
        self.cast_ns(Resume);
    }

    fn suspend(&self) {
        self.cast_ns(Suspend);
    }

    fn parent(&self) -> Option<&dyn TActorRef> {
        self.parent.as_ref().map(|actor_ref| &***actor_ref)
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        fn rec(
            actor: ActorRef,
            names: &mut Peekable<&mut dyn Iterator<Item = &str>>,
        ) -> Option<ActorRef> {
            match actor.local() {
                None => actor.get_child(names),
                Some(l) => {
                    let name = names.next();
                    let next = match name {
                        None => {
                            return Some(actor);
                        }
                        Some(name) => match name {
                            ".." => l.parent().map(|p| dyn_clone::clone_box(p).into()),
                            "" => Some(actor),
                            _ => l.get_single_child(name),
                        },
                    };
                    match next {
                        None => None,
                        Some(next) => rec(next, names),
                    }
                }
            }
        }
        rec(self.clone().into(), names)
    }
}

impl Into<ActorRef> for LocalActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl LocalActorRef {
    pub(crate) fn new(
        path: ActorPath,
        sender: MailboxSender,
        parent: Option<ActorRef>,
    ) -> (Self, SignalReceiver) {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let inner = LocalActorRefInner {
            path,
            sender,
            parent,
            children: DashMap::with_hasher(RandomState::default()),
            function_refs: DashMap::with_hasher(RandomState::default()),
            signal: tx,
        };
        (LocalActorRef(inner.into()), rx)
    }

    fn handle_result(&self, result: Result<(), TrySendError<Envelope>>) {
        if let Some(error) = result.err() {
            let actor_ref: ActorRef = self.clone().into();
            match error {
                TrySendError::Full(envelop) => {
                    let name = envelop.name();
                    match &envelop.sender {
                        None => {
                            warn!(
                                "message {} to {} was not delivered because mailbox is full",
                                name, actor_ref
                            );
                        }
                        Some(sender) => {
                            warn!(
                            "message {} from {} to {} was not delivered because mailbox is full",
                            name, sender, actor_ref
                        );
                        }
                    }
                }
                TrySendError::Closed(envelop) => {
                    let name = envelop.name();
                    match &envelop.sender {
                        None => {
                            warn!(
                                "message {} to {} was not delivered because actor stopped",
                                name, actor_ref
                            );
                        }
                        Some(sender) => {
                            warn!(
                                "message {} from {} to {} was not delivered because actor stopped",
                                name, sender, actor_ref
                            );
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn attach_child(
        &self,
        props: Props,
        system: ActorSystem,
        name: Option<String>,
        uid: Option<i32>,
    ) -> anyhow::Result<ActorRef> {
        let actor_spawn = self.attach_child_deferred(props, name, uid)?;
        let myself = actor_spawn.myself.clone();
        actor_spawn.spawn(system)?;
        Ok(myself)
    }

    pub(crate) fn attach_child_deferred(
        &self,
        props: Props,
        name: Option<String>,
        uid: Option<i32>,
    ) -> anyhow::Result<ActorSpawn> {
        //TODO mailbox
        let mailbox = crate::config::mailbox::Mailbox {
            mailbox_capacity: None,
            mailbox_push_timeout_time: Duration::from_days(1),
            stash_capacity: None,
            throughput: 100,
        };
        let (sender, mailbox) = props.mailbox(mailbox)?;
        let (child_ref, signal_rx) = self.make_child(name, uid, sender)?;
        let actor_spawn = ActorSpawn::new(props, child_ref.clone(), signal_rx, mailbox);
        Ok(actor_spawn)
    }

    pub(crate) fn make_child(
        &self,
        name: Option<String>,
        uid: Option<i32>,
        sender: MailboxSender,
    ) -> anyhow::Result<(ActorRef, SignalReceiver)> {
        if let Some(name) = &name {
            if name.is_empty() {
                return Err(anyhow!("name cannot be empty"));
            }
            check_name(name)?;
        }
        let name = name.unwrap_or_else(random_actor_name);
        let uid = uid.unwrap_or_else(ActorPath::new_uid);
        let path = ChildActorPath::new(self.path.clone(), name.clone(), uid).into();
        if self.children.contains_key(&name) {
            return Err(anyhow!("duplicate actor name {}", name));
        }
        let (child_ref, signal) = LocalActorRef::new(path, sender, Some(self.clone().into()));
        self.children.insert(name, child_ref.clone().into());
        Ok((child_ref.into(), signal))
    }

    pub(crate) fn get_child_by_name(&self, name: &str) -> Option<ActorRef> {
        self.children.get(name).map(|c| c.clone())
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
                    None => self.get_function_ref(name, uid).map(|f| f.into()),
                }
            }
            None => match self.get_child_by_name(name) {
                Some(child_ref) => Some(child_ref),
                None => self
                    .get_function_ref(name, ActorPath::undefined_uid())
                    .map(|f| f.into()),
            },
        }
    }

    pub(crate) fn remove_child(&self, name: &str) -> Option<ActorRef> {
        match self.children.remove(name) {
            None => None,
            Some((_, child)) => Some(child),
        }
    }

    pub(crate) fn add_function_ref<F>(&mut self, transform: F, name: Option<String>) -> FunctionRef
    where
        F: Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static,
    {
        let mut n = random_name("$$".to_string());
        if let Some(name) = name {
            n.push('-');
            n.push_str(&name);
        }
        let child_path = ChildActorPath::new(self.path().clone(), n, ActorPath::new_uid());
        let name = child_path.name().to_string();
        let function_ref = FunctionRef::new(child_path, transform);
        self.function_refs.insert(name, function_ref.clone());
        function_ref
    }

    pub(crate) fn remove_function_ref(&self, name: &str) -> Option<(String, FunctionRef)> {
        self.function_refs.remove(name)
    }

    pub(crate) fn get_function_ref(&self, name: &str, uid: i32) -> Option<FunctionRef> {
        match self.function_refs.get(name) {
            Some(function_ref)
                if uid == ActorPath::undefined_uid() || uid == function_ref.path.uid() =>
            {
                Some(function_ref.value().clone().into())
            }
            _ => None,
        }
    }
}
