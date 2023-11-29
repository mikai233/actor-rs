use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use crate::{DynMessage, MessageType};
use crate::actor_path::ActorPath;
use crate::actor_path::ChildActorPath;
use crate::actor_ref::{ActorRef, ActorRefSystemExt, TActorRef};
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::ext::{check_name, random_actor_name};
use crate::message::poison_pill::PoisonPill;
use crate::net::mailbox::MailboxSender;
use crate::props::Props;
use crate::system::ActorSystem;

use super::Cell;

#[derive(Clone)]
pub struct LocalActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) sender: MailboxSender,
    pub(crate) cell: ActorCell,
}

impl Deref for LocalActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for LocalActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .field("sender", &self.sender)
            .field("cell", &self.cell)
            .finish()
    }
}

impl TActorRef for LocalActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let envelop = Envelope { message, sender };
        match &envelop.message.message_type {
            MessageType::User | MessageType::AsyncUser => {
                if let Some(error) = self.sender.message.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            MessageType::System => {
                if let Some(error) = self.sender.system.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            MessageType::Untyped => {
                let myself: ActorRef = self.clone().into();
                warn!("unexpected Untyped message {} to {}", envelop.message.name, myself);
            }
        }
    }

    fn stop(&self) {
        self.cast_system(PoisonPill, None)
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.cell.parent()
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        fn rec(actor: ActorRef, mut names: impl Iterator<Item=String>) -> Option<ActorRef> {
            match &actor {
                ActorRef::LocalActorRef(l) => {
                    let name = names.next();
                    let next = match name {
                        None => {
                            return Some(actor);
                        }
                        Some(name) => {
                            match name.as_str() {
                                ".." => l.parent().cloned(),
                                "" => Some(actor),
                                _ => { l.get_single_child(&name) }
                            }
                        }
                    };
                    match next {
                        None => None,
                        Some(next) => { rec(next, names) }
                    }
                }
                _ => actor.get_child(names)
            }
        }
        rec(self.clone().into(), names.into_iter())
    }
}

impl Cell for LocalActorRef {
    fn underlying(&self) -> ActorCell {
        self.cell.clone()
    }

    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>> {
        self.cell.children()
    }

    fn get_single_child(&self, name: &String) -> Option<ActorRef> {
        match self.cell.get_single_child(name) {
            None => {
                self.cell.get_function_ref(name).map(|r| r.into())
            }
            Some(child) => { Some(child) }
        }
    }
}

impl LocalActorRef {
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

    pub(crate) fn attach_child(&self, props: Props, name: Option<String>) -> anyhow::Result<ActorRef> {
        if let Some(name) = &name {
            check_name(name)?;
        }
        let name = name.unwrap_or(random_actor_name());
        self.make_child(props, name)
    }

    pub(crate) fn make_child(&self, props: Props, name: String) -> anyhow::Result<ActorRef> {
        let (sender, mailbox) = props.mailbox();
        let path = ChildActorPath::new(self.path.clone(), name.clone(), ActorPath::new_uid()).into();
        let inner = Inner {
            system: self.system(),
            path,
            sender,
            cell: ActorCell::new(Some(self.clone().into())),
        };
        let child_ref = LocalActorRef {
            inner: inner.into(),
        };
        let mut children = self.children().write().unwrap();
        if children.contains_key(&name) {
            return Err(anyhow!("duplicate actor name {}", name));
        }
        children.insert(name, child_ref.clone().into());
        (props.spawner)(child_ref.clone().into(), mailbox, self.system());
        Ok(child_ref.into())
    }
}