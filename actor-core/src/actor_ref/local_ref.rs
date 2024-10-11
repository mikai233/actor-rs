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
use crate::actor::mailbox::MailboxSender;
use crate::actor::props::Props;
use crate::actor_path::child_actor_path::ChildActorPath;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::cell::envelope::Envelope;
use crate::ext::{check_name, random_actor_name};
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::message::DynMessage;
use crate::provider::TActorRefProvider;

pub type SignalSender = tokio::sync::mpsc::Sender<()>;

pub type SignalReceiver = tokio::sync::mpsc::Receiver<()>;

#[derive(Clone, derive_more::Deref, AsAny)]
pub struct LocalActorRef(Arc<LocalActorRefInner>);

pub struct LocalActorRefInner {
    pub(crate) path: ActorPath,
    pub(crate) sender: MailboxSender,
    pub(crate) parent: Option<ActorRef>,
    pub(crate) children: DashMap<String, ActorRef, RandomState>,
    pub(crate) signal: SignalSender,
}

impl Debug for LocalActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalActorRef")
            .field("path", &self.path)
            .field("sender", &self.sender)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("signal", &self.signal)
            .finish()
    }
}

impl TActorRef for LocalActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        unimplemented!("LocalActorRef.tell")
    }

    fn start(&self) {
        let _ = self.signal.try_send(());
    }

    fn stop(&self) {
        self.cast_system(PoisonPill, None)
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
                            ".." => l.parent().cloned(),
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
            signal: tx,
        };
        (LocalActorRef(inner.into()), rx)
    }

    fn log_send_error(&self, error: TrySendError<Envelope>) {
        let actor: ActorRef = self.clone().into();
        match error {
            TrySendError::Full(envelop) => {
                let name = envelop.name();
                match &envelop.sender {
                    None => {
                        warn!(
                            "message {} to {} was not delivered because mailbox is full",
                            name, actor
                        );
                    }
                    Some(sender) => {
                        warn!(
                            "message {} from {} to {} was not delivered because mailbox is full",
                            name, sender, actor
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
                            name, actor
                        );
                    }
                    Some(sender) => {
                        warn!(
                            "message {} from {} to {} was not delivered because actor stopped",
                            name, sender, actor
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn attach_child(
        &self,
        props: Props,
        system: ActorSystem,
        provider: impl AsRef<dyn TActorRefProvider>,
        name: Option<String>,
        uid: Option<i32>,
    ) -> anyhow::Result<ActorRef> {
        let provider = provider.as_ref();
        let mailbox: crate::config::mailbox::Mailbox = match props.mailbox.as_ref() {
            None => provider
                .settings()
                .cfg
                .get("akka.actor.mailbox.default-mailbox")?,
            Some(mailbox_name) => provider
                .settings()
                .cfg
                .get(&format!("akka.actor.mailbox.{}", mailbox_name))?,
        };
        let (sender, mailbox) = props.mailbox(mailbox)?;
        let (child_ref, signal) = self.make_child(name, uid, sender)?;
        props.spawn(child_ref.clone(), signal, mailbox, system)?;
        Ok(child_ref)
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
                    None => None,
                }
            }
            None => self.get_child_by_name(name),
        }
    }

    pub(crate) fn remove_child(&self, name: &str) -> Option<ActorRef> {
        match self.children.remove(name) {
            None => None,
            Some((_, child)) => Some(child),
        }
    }
}
