use tokio::task::yield_now;
use tracing::error;

use crate::{Actor, AsyncMessage, Message};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::actor::mailbox::Mailbox;
use crate::actor::props::Props;
use crate::actor::state::ActorState;
use crate::cell::envelope::Envelope;
use crate::delegate::MessageDelegate;

pub struct ActorRuntime<A> where A: Actor {
    pub(crate) actor: A,
    pub(crate) context: ActorContext,
    pub(crate) mailbox: Mailbox,
    pub(crate) props: Props,
}

impl<A> ActorRuntime<A> where A: Actor {
    pub(crate) async fn run(self) {
        let Self { mut actor, mut context, mut mailbox, props } = self;
        let actor_name = std::any::type_name::<A>();
        if let Err(err) = actor.pre_start(&mut context).await {
            error!("actor {:?} pre start error {:?}", actor_name, err);
            context.stop(&context.myself());
            while let Some(message) = mailbox.system.recv().await {
                Self::handle_system(&mut context, &mut actor, message).await;
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
                    Self::handle_system(&mut context, &mut actor, message).await;
                    if matches!(context.state, ActorState::CanTerminate | ActorState::Recreate) {
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
        if matches!(context.state, ActorState::Recreate) {
            if let Some(err) = actor.pre_restart(&mut context).await.err() {
                error!("actor {:?} pre restart error {:?}", actor_name, err);
            }
            let spawner = props.spawner.clone();
            spawner(context.myself, mailbox, context.system, props);
        } else {
            if let Some(err) = actor.post_stop(&mut context).await.err() {
                error!("actor {:?} post stop error {:?}", actor_name, err);
            }
            mailbox.close();
            context.state = ActorState::Terminated;
        }
        for task in context.async_tasks {
            task.abort();
        }
    }

    async fn handle_system(context: &mut ActorContext, actor: &mut A, envelope: Envelope) {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_into_delegate::<A>() {
            Ok(delegate) => {
                match delegate {
                    MessageDelegate::User(m) => {
                        panic!("unexpected user message {} in system handle", m.name);
                    }
                    MessageDelegate::AsyncUser(m) => {
                        panic!("unexpected async user message {} in system handle", m.name);
                    }
                    MessageDelegate::System(system) => {
                        if let Some(error) = system.message.handle(context, actor).await.err() {
                            let myself = context.myself.clone();
                            context.handle_invoke_failure(myself, error);
                        }
                    }
                }
            }
            Err(error) => {
                error!("{:?}", error);
            }
        }
        context.sender.take();
    }

    async fn handle_message(context: &mut ActorContext, actor: &mut A, envelope: Envelope) -> bool {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_into_delegate::<A>() {
            Ok(delegate) => {
                match delegate {
                    MessageDelegate::User(user) => {
                        if let Some(error) = user.handle(context, actor).err() {
                            let myself = context.myself.clone();
                            context.handle_invoke_failure(myself, error);
                        }
                    }
                    MessageDelegate::AsyncUser(user) => {
                        if let Some(error) = user.handle(context, actor).await.err() {
                            let myself = context.myself.clone();
                            context.handle_invoke_failure(myself, error);
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