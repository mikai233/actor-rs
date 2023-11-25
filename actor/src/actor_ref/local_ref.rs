use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::sync::RwLock;

use anyhow::anyhow;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use crate::{Actor, DynamicMessage};
use crate::actor_path::{ChildActorPath};
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, ActorRefSystemExt, TActorRef};
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::cell::runtime::ActorRuntime;
use crate::ext::{check_name, random_actor_name};
use crate::message::poison_pill::PoisonPill;
use crate::net::mailbox::{Mailbox, MailboxSender};
use crate::props::Props;
use crate::system::ActorSystem;

use super::Cell;

#[derive(Clone)]
pub struct LocalActorRef {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) sender: MailboxSender,
    pub(crate) cell: ActorCell,
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

    fn tell(&self, message: DynamicMessage, sender: Option<ActorRef>) {
        let envelop = Envelope { message, sender };
        match &envelop.message {
            DynamicMessage::User(_) | DynamicMessage::AsyncUser(_) => {
                if let Some(error) = self.sender.message.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            DynamicMessage::System(_) => {
                if let Some(error) = self.sender.system.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            DynamicMessage::Untyped(m) => {
                let myself: ActorRef = self.clone().into();
                warn!("unexpected Deferred message {} to {}", m.name(), myself);
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

    pub(crate) fn attach_child<T>(&self, actor: T, arg: T::A, name: Option<String>, props: Props) -> anyhow::Result<ActorRef> where T: Actor {
        if let Some(name) = &name {
            check_name(name)?;
        }
        let name = name.unwrap_or(random_actor_name());
        self.make_child(actor, arg, name, props)
    }

    pub(crate) fn make_child<T>(&self, actor: T, arg: T::A, name: String, props: Props) -> anyhow::Result<ActorRef> where T: Actor {
        let (sender, mailbox) = props.mailbox();
        let path = ChildActorPath::new(self.path.clone(), name.clone(), ActorPath::new_uid()).into();
        let child_ref = LocalActorRef {
            system: self.system(),
            path,
            sender,
            cell: ActorCell::new(Some(self.clone().into())),
        };
        let mut children = self.children().write().unwrap();
        if children.contains_key(&name) {
            return Err(anyhow!("duplicate actor name {}", name));
        }
        children.insert(name, child_ref.clone().into());
        child_ref.start(actor, arg, props, mailbox);
        Ok(child_ref.clone().into())
    }

    pub(crate) fn start<T>(&self, actor: T, arg: T::A, props: Props, mailbox: Mailbox) where T: Actor {
        let rt = ActorRuntime {
            myself: self.clone().into(),
            handler: actor,
            props,
            system: self.system(),
            mailbox,
            arg,
        };
        self.system.spawn(rt.run());
    }
}