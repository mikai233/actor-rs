use std::any::Any;
use std::any::type_name;
use std::panic::AssertUnwindSafe;

use anyhow::{anyhow, Error};
use futures::FutureExt;
use tokio::task::yield_now;
use tracing::error;

use crate::{Actor, Message};
use crate::actor::context::{ActorContext, Context};
use crate::actor::mailbox::Mailbox;
use crate::actor::state::ActorState;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::cell::envelope::Envelope;

pub struct ActorRuntime<A> where A: Actor {
    pub(crate) actor: A,
    pub(crate) context: ActorContext,
    pub(crate) mailbox: Mailbox,
}

impl<A> ActorRuntime<A> where A: Actor {
    pub(crate) async fn run(self) {
        let Self { mut actor, mut context, mut mailbox } = self;
        context.stash_capacity = mailbox.stash_capacity;
        let actor_name = std::any::type_name::<A>();
        if let Err(err) = actor.started(&mut context).await {
            error!("actor {} start error {:#?}", actor_name, err);
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
                    if matches!(context.state, ActorState::CanTerminate) {
                        break;
                    }
                    context.remove_finished_tasks();
                }
                Some(message) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    match AssertUnwindSafe(Self::handle_message(&mut context, &mut actor, message)).catch_unwind().await {
                        Ok(should_break) => {
                            if should_break {
                                break;
                            }
                        }
                        Err(_) => {
                            Self::handle_failure(&mut context, anyhow!("{} panic", type_name::<A>()));
                        }
                    }
                    throughput += 1;
                    if throughput >= mailbox.throughput {
                        throughput = 0;
                        context.remove_finished_tasks();
                        yield_now().await;
                    }
                }
                else  => {
                    break;
                }
            }
        }
        if let Some(err) = actor.stopped(&mut context).await.err() {
            error!("actor {} stop error {:?}", actor_name, err);
        }
        mailbox.close();
        context.state = ActorState::Terminated;
        for task in context.fut_handle {
            task.abort();
        }
    }

    async fn handle_system(context: &mut ActorContext, actor: &mut A, envelope: Envelope) {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        match message.downcast_system_delegate() {
            Ok(system) => {
                let catch_unwind_result = AssertUnwindSafe(system.message.handle(context, actor)).catch_unwind().await;
                Self::catch_handle_error(context, catch_unwind_result);
            }
            Err(error) => {
                error!("{:?}", error);
            }
        }
        context.sender.take();
    }

    fn catch_handle_error(context: &mut ActorContext, catch_unwind_result: Result<anyhow::Result<()>, Box<dyn Any + Send>>) {
        match catch_unwind_result {
            Ok(Err(logic_error)) => {
                let name = std::any::type_name::<A>();
                let myself = context.myself.clone();
                context.handle_invoke_failure(myself, name, logic_error);
            }
            Err(_) => {
                let name = std::any::type_name::<A>();
                let myself = context.myself.clone();
                context.handle_invoke_failure(myself, name, anyhow!("{} panic", name));
            }
            _ => {}
        }
    }

    async fn handle_message(context: &mut ActorContext, actor: &mut A, envelope: Envelope) -> bool {
        let Envelope { message, sender } = envelope;
        context.sender = sender;
        if let Some(message) = actor.on_recv(context, message) {
            match message.downcast_user_delegate::<A>() {
                Ok(message) => {
                    if let Some(error) = message.handle(context, actor).await.err() {
                        Self::handle_failure(context, error);
                    }
                }
                Err(error) => {
                    error!("{:?}", error);
                }
            }
        }
        context.sender.take();
        return false;
    }

    fn handle_failure(context: &mut ActorContext, error: Error) {
        let name = type_name::<A>();
        let myself = context.myself.clone();
        context.handle_invoke_failure(myself, name, error);
    }
}