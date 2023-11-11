use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::Duration;

use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use crate::actor::{DynamicMessage, Message};
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::net::mailbox::MailboxSender;
use crate::system::ActorSystem;

use super::Cell;

#[derive(Debug, Clone)]
pub struct LocalActorRef {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) sender: MailboxSender,
    pub(crate) cell: ActorCell,
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
        if matches!(envelop.message, DynamicMessage::System(_)) {
            if let Some(error) = self.sender.system.try_send(envelop).err() {
                self.report_send_error(error);
            }
        } else {
            if let Some(error) = self.sender.message.try_send(envelop).err() {
                self.report_send_error(error);
            }
        }
    }

    fn stop(&self) {
        //TODO
        // let terminate = ActorRemoteSystemMessage::Terminate;
        // let terminate = ActorMessage::remote_system(terminate);
        // let sender = Some(self.clone().into());
        // self.tell(terminate, sender);
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
        self.cell.get_single_child(name)
    }
}

impl LocalActorRef {
    fn report_send_error(&self, error: TrySendError<Envelope>) {
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
}

pub async fn ask<M, R>(actor: &LocalActorRef, message: M, timeout: Duration) -> anyhow::Result<R>
    where
        M: Message,
        R: Message,
{
    todo!()
}
