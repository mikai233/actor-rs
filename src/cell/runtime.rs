use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use futures::FutureExt;
use futures::stream::FuturesUnordered;
use tokio::task::yield_now;
use tracing::error;

use crate::actor::{Actor, Message};
use crate::actor::context::{ActorContext, ActorThreadPoolMessage, Context};
use crate::actor_ref::ActorRef;
use crate::cell::envelope::Envelope;
use crate::message::{
    ActorLocalMessage, ActorMessage, ActorRemoteMessage, ActorRemoteSystemMessage,
    ActorSystemMessage,
};
use crate::net::mailbox::Mailbox;
use crate::props::Props;
use crate::provider::ActorRefFactory;
use crate::state::ActorState;
use crate::system::ActorSystem;

use super::envelope::UserEnvelope;

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
            _phantom: PhantomData::default(),
            state: ActorState::Init,
            myself,
            sender: None,
            stash: VecDeque::new(),
            tasks: Vec::new(),
            system,
        };
        let mut state = match handler.pre_start(&mut context, arg) {
            Ok(state) => state,
            Err(err) => {
                error!("actor {:?} pre start error {:?}", actor, err);
                context.stop(&context.myself());
                while let Some(message) = mailbox.signal.recv().await {
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
                Some(message) = mailbox.signal.recv() => {
                    Self::handle_system(&mut context, message).await;
                    if matches!(context.state, ActorState::CanTerminate) {
                        break;
                    }
                    context.remove_finished_task();
                }
                Some(message) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    if Self::handle_message(&mut context, &mut state, &handler, message) {
                        break;
                    }
                    context.remove_finished_task();
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
        for task in context.tasks {
            task.abort();
        }
        mailbox.close();
        context.state = ActorState::Terminated;
    }

    async fn handle_system(context: &mut ActorContext<T>, envelope: Envelope) {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message {
            ActorMessage::Local(l) => match l {
                ActorLocalMessage::User { .. } => panic!("unreachable user branch"),
                ActorLocalMessage::System { message } => match message {
                    ActorSystemMessage::DeathWatchNotification { actor } => {
                        context.watched_actor_terminated(actor);
                    }
                },
            },
            ActorMessage::Remote(r) => match r {
                ActorRemoteMessage::User { .. } => panic!("unreachable user branch"),
                ActorRemoteMessage::System { message } => match message {
                    ActorRemoteSystemMessage::Terminate => {
                        context.handle_terminate();
                    }
                    ActorRemoteSystemMessage::Terminated(_) => todo!(),
                    ActorRemoteSystemMessage::Watch { watchee, watcher } => todo!(),
                    ActorRemoteSystemMessage::UnWatch { watchee, watcher } => todo!(),
                },
            },
        }
        context.sender.take();
    }

    fn handle_message(
        context: &mut ActorContext<T>,
        state: &mut T::S,
        handler: &T,
        envelope: Envelope,
    ) -> bool {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        let user_envelope = match message {
            ActorMessage::Local(l) => match l {
                ActorLocalMessage::User { name, inner } => match T::M::downcast(inner) {
                    Ok(message) => UserEnvelope::Local(message),
                    Err(message) => UserEnvelope::Unknown { name, message },
                },
                ActorLocalMessage::System { .. } => panic!("unreachable system message branch"),
            },
            ActorMessage::Remote(r) => match r {
                ActorRemoteMessage::User { name, message } => {
                    UserEnvelope::Remote { name, message }
                }
                ActorRemoteMessage::System { message } => {
                    panic!("unreachable system message branch")
                }
            },
        };

        if let Some(error) = handler.on_recv(context, state, user_envelope).err() {
            let actor = std::any::type_name::<T>();
            error!("actor {} handle message error: {:?}", actor, error);
            return true;
        }
        context.sender.take();
        return false;
    }
}

impl<T> Into<ActorThreadPoolMessage> for ActorRuntime<T>
    where
        T: Actor,
{
    fn into(self) -> ActorThreadPoolMessage {
        let spawn_fn = move |futures: &mut FuturesUnordered<Pin<Box<dyn Future<Output=()>>>>| {
            futures.push(self.run().boxed_local());
        };
        ActorThreadPoolMessage::SpawnActor(Box::new(spawn_fn))
    }
}
