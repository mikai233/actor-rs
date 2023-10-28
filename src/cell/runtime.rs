use std::collections::VecDeque;
use std::sync::Arc;

use tokio::task::yield_now;
use tracing::error;

use crate::actor::{Actor, Message};
use crate::actor::context::{ActorContext, Context};
use crate::actor_ref::ActorRef;
use crate::cell::ActorCell;
use crate::cell::envelope::Envelope;
use crate::message::{ActorMessage, Signal};
use crate::net::mailbox::Mailbox;
use crate::props::Props;
use crate::provider::ActorRefFactory;
use crate::state::ActorState;
use crate::system::ActorSystem;

pub struct ActorRuntime<T> where T: Actor {
    pub(crate) actor_ref: ActorRef,
    pub(crate) handler: T,
    pub(crate) props: Props,
    pub(crate) system: ActorSystem,
    pub(crate) parent: Option<Arc<ActorCell>>,
    pub(crate) mailbox: Mailbox,
    pub(crate) arg: T::A,
}

impl<T> ActorRuntime<T> where T: Actor {
    pub(crate) async fn run(self) {
        let Self {
            actor_ref,
            handler,
            props,
            system,
            parent,
            mut mailbox,
            arg
        } = self;
        let cell = ActorCell {
            parent,
            myself: actor_ref,
            children: Arc::new(Default::default()),
        };
        let mut context = ActorContext {
            state: ActorState::Init,
            cell,
            sender: None,
            stash: VecDeque::new(),
            tasks: Vec::new(),
            system,
        };
        let mut state = match handler.pre_start(&mut context, arg) {
            Ok(state) => state,
            Err(err) => {
                error!("actor {:?} pre start error {:?}",handler,err);
                context.stop(context.myself());
                while let Some(message) = mailbox.signal.recv().await {
                    if Self::handle_signal(&mut context, message).await {
                        break;
                    }
                }
                return;
            }
        };
        context.state = ActorState::Started;
        let mut throughput = 0;
        loop {
            tokio::select! {
                Some(message) = mailbox.signal.recv() => {
                    if Self::handle_signal(&mut context, message).await {
                        break;
                    }
                }
                Some(message) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    if Self::handle_message(&mut context, &mut state, &handler, message) {
                        break;
                    }
                    throughput += 1;
                    if throughput >= props.throughput {
                        throughput = 0;
                        yield_now().await;
                    }
                }
                else  => {
                    break;
                }
            }
        }
        if let Some(err) = handler.post_stop(&mut context, &mut state).err() {
            error!("actor {:?} post stop error {:?}",handler,err);
        }
        for task in context.tasks {
            task.abort();
        }
        mailbox.close();
        context.state = ActorState::Stopped;
    }

    async fn handle_signal(context: &mut ActorContext<T>, signal: Signal) -> bool {
        async fn signal_parent<T>(context: &mut ActorContext<T>, signal: Signal) where T: Actor {
            if let Some(parent) = context.parent().clone() {
                // parent.signal(signal).await;
            }
        }
        match signal {
            Signal::Stop => {
                context.state = ActorState::Stopping;
                if context.children().is_empty() {
                    // signal_parent(context, Signal::Terminated(context.myself.clone())).await;
                    return true;
                } else {
                    for child in context.children().values() {
                        // child.signal(SignalMessage::Stop).await;
                    }
                    context.state = ActorState::WaitingChildrenStop;
                }
            }
            Signal::Terminated(who) => {
                // context.children.retain(|child| { *child != who });
                // if matches!(context.state, ActorState::WaitingChildrenStop) {
                //     if context.children.is_empty() {
                //         signal_parent(context, SignalMessage::Terminated(context.myself.untyped())).await;
                //         return true;
                //     }
                // }
            }
        }
        return false;
    }

    fn handle_message(context: &mut ActorContext<T>, state: &mut T::S, handler: &T, envelop: Envelope) -> bool {
        let Envelope { message, sender } = envelop;
        context.sender = sender;
        let message = match message {
            ActorMessage::Local(l) => {
                match T::M::downcast(l.inner) {
                    Ok(message) => {
                        message
                    }
                    Err(message) => {
                        match handler.transform(message) {
                            None => {
                                error!("actor {:?} handle unexpected message",handler);
                                return false;
                            }
                            Some(message) => message
                        }
                    }
                }
            }
            ActorMessage::Remote(r) => {
                todo!()
            }
        };

        if let Some(error) = handler.on_recv(context, state, message).err() {
            error!("actor: {:?} handle message error: {:?}",handler,error);
            return true;
        }
        context.sender.take();
        return false;
    }
}