use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::Duration;

use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use crate::actor::Message;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::message::ActorLocalMessage;
use crate::message::ActorMessage;
use crate::message::ActorRemoteMessage;
use crate::message::ActorRemoteSystemMessage;
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

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        let envelop = Envelope { message, sender };
        match &envelop.message {
            ActorMessage::Local(l) => match l {
                ActorLocalMessage::User { .. } => {
                    if let Some(error) = self.sender.message.try_send(envelop).err() {
                        self.report_send_error(error);
                    }
                }
                ActorLocalMessage::System { .. } => {
                    if let Some(error) = self.sender.signal.try_send(envelop).err() {
                        self.report_send_error(error);
                    }
                }
            },
            ActorMessage::Remote(r) => match r {
                ActorRemoteMessage::User { .. } => {
                    if let Some(error) = self.sender.message.try_send(envelop).err() {
                        self.report_send_error(error);
                    }
                }
                ActorRemoteMessage::System { .. } => {
                    if let Some(error) = self.sender.signal.try_send(envelop).err() {
                        self.report_send_error(error);
                    }
                }
            },
        }
    }

    fn stop(&self) {
        let terminate = ActorRemoteSystemMessage::Terminate;
        let terminate = ActorMessage::remote_system(terminate);
        let sender = Some(self.clone().into());
        self.tell(terminate, sender);
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
    // let actor_info = format!("{:?}", actor);
    // let message_info = format!("{:?}", message);
    // let (tx, rx) = tokio::sync::oneshot::channel();
    // #[derive(Debug)]
    // struct Waiter;
    //
    // #[derive(Debug)]
    // enum WaiterMessage {
    //     Response(Box<dyn Any + Send + 'static>),
    //     Timeout,
    // }
    //
    // type Sender = tokio::sync::oneshot::Sender<Box<dyn Any + Send + 'static>>;
    //
    // impl Actor for Waiter {
    //     type M = WaiterMessage;
    //     type S = Option<Sender>;
    //     type A = (Sender, Duration);
    //
    //     fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
    //         let (sender, timeout) = arg;
    //         let myself = ctx.myself.clone();
    //         ctx.spawn_task(async move {
    //             tokio::time::sleep(timeout).await;
    //             myself.tell(WaiterMessage::Timeout, None);
    //         });
    //         Ok(Some(sender))
    //     }
    //
    //     fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
    //         match message {
    //             WaiterMessage::Response(message) => {
    //                 let _ = state.take().unwrap().send(message);
    //             }
    //             WaiterMessage::Timeout => {
    //                 ctx.stop();
    //             }
    //         }
    //         Ok(())
    //     }
    //
    //     fn transform(&self, message: Box<dyn Any + Send + 'static>) -> Option<Self::M> {
    //         Some(WaiterMessage::Response(message))
    //     }
    // }
    // let config = ActorConfig {
    //     mailbox: 1
    // };
    // let waiter = actor_of(config, Waiter, (tx, timeout))?;
    // let future = async {
    //     actor.tell(message, Some(waiter.untyped()));
    //     let resp = rx.await?;
    //     Ok::<Box<dyn Any + Send + 'static>, anyhow::Error>(resp)
    // };
    // let response = tokio::time::timeout(timeout, future).await
    //     .map_err(|_| { anyhow!("ask message {} to actor {} timeout after {:?}",message_info,actor_info,timeout) })??;
    // let response = R::downcast(response)
    //     .map_err(|_| { anyhow!("ask message {} to actor {} got wrong response type",message_info,actor_info) })?;
    // Ok(response)
}
