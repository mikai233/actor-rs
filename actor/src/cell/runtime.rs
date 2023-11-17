use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;

use tokio::task::yield_now;
use tracing::error;

use crate::actor::{Actor, AsyncMessage, Message};
use crate::actor::context::{ActorContext, Context};
use crate::actor_ref::ActorRef;
use crate::cell::envelope::Envelope;
use crate::delegate::MessageDelegate;
use crate::net::mailbox::Mailbox;
use crate::props::Props;
use crate::provider::ActorRefFactory;
use crate::state::ActorState;
use crate::system::ActorSystem;

pub struct ActorRuntime<T>
    where
        T: Actor,
{
    pub(crate) myself: ActorRef,
    pub(crate) handler: T,
    pub(crate) props: Props,
    pub(crate) system: ActorSystem,
    pub(crate) mailbox: Mailbox,
    pub(crate) arg: T::A,
}

impl<T> ActorRuntime<T>
    where
        T: Actor,
{
    pub(crate) async fn run(self) {
        let Self {
            myself,
            handler,
            props,
            system,
            mut mailbox,
            arg,
        } = self;
        let actor = std::any::type_name::<T>();
        let mut context = ActorContext {
            state: ActorState::Init,
            myself,
            sender: None,
            stash: VecDeque::new(),
            async_tasks: Vec::new(),
            system,
            watching: HashMap::new(),
            watched_by: HashSet::new(),
        };
        let mut state = match handler.pre_start(&mut context, arg) {
            Ok(state) => state,
            Err(err) => {
                error!("actor {:?} pre start error {:?}", actor, err);
                context.stop(&context.myself());
                while let Some(message) = mailbox.system.recv().await {
                    Self::handle_system(&mut context, message).await;
                    if matches!(context.state, ActorState::CanTerminate) {
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
                biased;
                Some(message) = mailbox.system.recv() => {
                    Self::handle_system(&mut context, message).await;
                    if matches!(context.state, ActorState::CanTerminate) {
                        break;
                    }
                    context.remove_finished_tasks();
                }
                Some(message) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    if Self::handle_message(&mut context, &mut state, message).await {
                        break;
                    }
                    context.remove_finished_tasks();
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
            error!("actor {:?} post stop error {:?}", actor, err);
        }
        for task in context.async_tasks {
            task.abort();
        }
        mailbox.close();
        context.state = ActorState::Terminated;
    }

    async fn handle_system(context: &mut ActorContext, envelope: Envelope) {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_into_delegate::<T>() {
            Ok(delegate) => {
                match delegate {
                    MessageDelegate::User(m) => {
                        panic!("unexpected user message {} in system handle", m.name);
                    }
                    MessageDelegate::AsyncUser(m) => {
                        panic!("unexpected async user message {} in system handle", m.name);
                    }
                    MessageDelegate::System(system) => {
                        if let Some(e) = system.message.handle(context).await.err() {
                            let actor_name = std::any::type_name::<T>();
                            error!("{} {:?}", actor_name, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }
        context.sender.take();
    }

    async fn handle_message(context: &mut ActorContext, state: &mut T::S, envelope: Envelope) -> bool {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_into_delegate::<T>() {
            Ok(delegate) => {
                match delegate {
                    MessageDelegate::User(user) => {
                        if let Some(e) = user.handle(context, state).err() {
                            let actor_name = std::any::type_name::<T>();
                            error!("{} {:?}", actor_name, e);
                        }
                    }
                    MessageDelegate::AsyncUser(user) => {
                        if let Some(e) = user.handle(context, state).await.err() {
                            let actor_name = std::any::type_name::<T>();
                            error!("{} {:?}", actor_name, e);
                        }
                    }
                    MessageDelegate::System(m) => {
                        panic!("unexpected system message {} in user handle", m.name);
                    }
                }
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }
        context.sender.take();
        return false;
    }
}