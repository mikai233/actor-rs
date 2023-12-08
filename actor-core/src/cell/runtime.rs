use tokio::task::yield_now;
use tracing::error;

use crate::{Actor, AsyncMessage, Message};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::actor::mailbox::Mailbox;
use crate::cell::envelope::Envelope;
use crate::delegate::MessageDelegate;
use crate::actor::state::ActorState;

pub struct ActorRuntime<A> where A: Actor {
    pub(crate) actor: A,
    pub(crate) context: ActorContext,
    pub(crate) mailbox: Mailbox,
}

impl<T> ActorRuntime<T> where T: Actor {
    pub(crate) async fn run(self) {
        let Self { mut actor, mut context, mut mailbox } = self;
        let actor_name = std::any::type_name::<T>();
        if let Err(err) = actor.pre_start(&mut context).await {
            error!("actor {:?} pre start error {:?}", actor_name, err);
            context.stop(&context.myself());
            while let Some(message) = mailbox.system.recv().await {
                Self::handle_system(&mut context, message).await;
                if matches!(context.state, ActorState::CanTerminate) {
                    break;
                }
            }
            return;
        }
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
                    if Self::handle_message(&mut context, &mut actor, message).await {
                        break;
                    }
                    context.remove_finished_tasks();
                    throughput += 1;
                    if throughput >= mailbox.throughput {
                        throughput = 0;
                        yield_now().await;
                    }
                }
                else  => {
                    break;
                }
            }
        }
        if let Some(err) = actor.post_stop(&mut context).await.err() {
            error!("actor {:?} post stop error {:?}", actor_name, err);
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

    async fn handle_message(context: &mut ActorContext, actor: &mut T, envelope: Envelope) -> bool {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_into_delegate::<T>() {
            Ok(delegate) => {
                match delegate {
                    MessageDelegate::User(user) => {
                        if let Some(e) = user.handle(context, actor).err() {
                            let actor_name = std::any::type_name::<T>();
                            error!("{} {:?}", actor_name, e);
                        }
                    }
                    MessageDelegate::AsyncUser(user) => {
                        if let Some(e) = user.handle(context, actor).await.err() {
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