use std::fmt::{Display, Formatter};
use std::time::Duration;

use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use crate::actor::Message;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::envelope::Envelope;
use crate::message::ActorMessage;
use crate::net::mailbox::MailboxSender;

#[derive(Debug, Clone)]
pub struct LocalActorRef {
    path: ActorPath,
    sender: MailboxSender,
}

impl TActorRef for LocalActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        let envelop = Envelope {
            message,
            sender,
        };
        if let Some(error) = self.sender.message.try_send(envelop).err() {
            match error {
                TrySendError::Full(envelop) => {
                    let name = envelop.name();
                    match envelop.sender {
                        None => {
                            warn!("message {} to {} was not delivered because mailbox is full",name,self);
                        }
                        Some(sender) => {
                            warn!("message {} from {} to {} was not delivered because mailbox is full",name,sender,self);
                        }
                    }
                }
                TrySendError::Closed(envelop) => {
                    let name = envelop.name();
                    match envelop.sender {
                        None => {
                            warn!("message {} to {} was not delivered because actor stopped",name,self);
                        }
                        Some(sender) => {
                            warn!("message {} from {} to {} was not delivered because actor stopped",name,sender,self);
                        }
                    }
                }
            }
        }
    }
}

impl Display for LocalActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let uid = path.uid();
        write!(f, "Actor[{}#{}]", path, uid)
    }
}

pub async fn ask<M, R>(actor: &LocalActorRef, message: M, timeout: Duration) -> anyhow::Result<R> where M: Message, R: Message {
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