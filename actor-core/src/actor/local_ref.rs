use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::anyhow;
use dashmap::DashMap;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use actor_derive::AsAny;

use crate::{DynMessage, MessageType};
use crate::actor::actor_path::ActorPath;
use crate::actor::actor_path::child_actor_path::ChildActorPath;
use crate::actor::actor_ref::{ActorRefSystemExt, TActorRef};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_selection::{ActorSelection, ActorSelectionMessage};
use crate::actor::actor_system::ActorSystem;
use crate::actor::cell::Cell;
use crate::actor::mailbox::MailboxSender;
use crate::actor::props::{ActorDeferredSpawn, Props};
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::ext::{check_name, random_actor_name};
use crate::message::poison_pill::PoisonPill;
use crate::message::recreate::Recreate;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::routing::router_config::TRouterConfig;

#[derive(Clone, AsAny)]
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
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        match &message.message_type {
            MessageType::User => {
                let envelop = Envelope { message, sender };
                if let Some(error) = self.sender.message.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            MessageType::System => {
                let envelop = Envelope { message, sender };
                if let Some(error) = self.sender.system.try_send(envelop).err() {
                    self.log_send_error(error);
                }
            }
            MessageType::Orphan => {
                if message.is::<ActorSelectionMessage>() {
                    let sel = message.downcast_orphan::<ActorSelectionMessage>().unwrap();
                    if sel.elements.is_empty() {
                        self.tell(sel.message, sender);
                    } else {
                        ActorSelection::deliver_selection(self.clone().into(), sender, sel);
                    }
                } else {
                    let myself: ActorRef = self.clone().into();
                    warn!("unexpected Orphan message {} to {}", message.name, myself);
                }
            }
        }
    }

    fn stop(&self) {
        self.cast_system(PoisonPill, None)
    }

    fn parent(&self) -> Option<&ActorRef> {
        self.cell.parent()
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        fn rec(actor: ActorRef, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
            match actor.local() {
                None => actor.get_child(names),
                Some(l) => {
                    let name = names.next();
                    let next = match name {
                        None => {
                            return Some(actor);
                        }
                        Some(name) => {
                            match name {
                                ".." => l.parent().cloned(),
                                "" => Some(actor),
                                _ => { l.get_single_child(name) }
                            }
                        }
                    };
                    match next {
                        None => None,
                        Some(next) => { rec(next, names) }
                    }
                }
            }
        }
        rec(self.clone().into(), names)
    }

    fn resume(&self) {
        self.cast_system(Resume, ActorRef::no_sender());
    }

    fn suspend(&self) {
        self.cast_system(Suspend, ActorRef::no_sender());
    }

    fn restart(&self) {
        self.cast_system(Recreate, ActorRef::no_sender());
    }
}

impl Cell for LocalActorRef {
    fn underlying(&self) -> ActorCell {
        self.cell.clone()
    }

    fn children(&self) -> &DashMap<String, ActorRef, ahash::RandomState> {
        self.cell.children()
    }

    fn get_single_child(&self, name: &str) -> Option<ActorRef> {
        match self.cell.get_single_child(name) {
            None => {
                self.cell.get_function_ref(name).map(|r| r.into())
            }
            Some(child) => { Some(child) }
        }
    }
}

impl Into<ActorRef> for LocalActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

impl LocalActorRef {
    pub(crate) fn new(system: ActorSystem, path: ActorPath, sender: MailboxSender, cell: ActorCell) -> Self {
        Self {
            inner: Arc::new(Inner {
                system,
                path,
                sender,
                cell,
            }),
        }
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

    pub fn attach_child(&self, props: Props, name: Option<String>, start: bool) -> anyhow::Result<(ActorRef, Option<ActorDeferredSpawn>)> {
        if let Some(name) = &name {
            check_name(name)?;
        }
        self.make_child(props, name, start)
    }

    pub(crate) fn make_child(&self, props: Props, name: Option<String>, start: bool) -> anyhow::Result<(ActorRef, Option<ActorDeferredSpawn>)> {
        let name_is_none = name.is_none();
        let name = name.unwrap_or(random_actor_name());
        let (sender, mailbox) = props.mailbox();
        let uid = if name == "system" || name == "user" {
            ActorPath::undefined_uid()
        } else {
            ActorPath::new_uid()
        };
        let path = ChildActorPath::new(self.path.clone(), name.clone(), uid).into();
        let children = self.children();
        if children.contains_key(&name) {
            return Err(anyhow!("duplicate actor name {}", name));
        }
        match props.router_config() {
            None => {
                let inner = Inner {
                    system: self.system().clone(),
                    path,
                    sender,
                    cell: ActorCell::new(Some(self.clone().into())),
                };
                let child_ref = LocalActorRef {
                    inner: inner.into(),
                };
                self.cell.insert_child(name, child_ref.clone());
                if start {
                    (props.spawner)(child_ref.clone().into(), mailbox, self.system().clone(), props.clone());
                    Ok((child_ref.into(), None))
                } else {
                    let deferred_spawn = ActorDeferredSpawn::new(child_ref.clone().into(), mailbox, props);
                    Ok((child_ref.into(), Some(deferred_spawn)))
                }
            }
            Some(router_config) => {
                let router_config = router_config.clone();
                let router_actor_props = Props::create(move |_| router_config.create_router_actor(props.clone()));
                if name_is_none {
                    self.attach_child(router_actor_props, None, start)
                } else {
                    self.attach_child(router_actor_props, Some(name), start)
                }
            }
        }
    }
}