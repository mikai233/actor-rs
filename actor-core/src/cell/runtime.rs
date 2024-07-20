use std::any::Any;
use std::any::type_name;
use std::panic::AssertUnwindSafe;

use anyhow::{anyhow, Error};
use futures::FutureExt;
use tokio::select;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::Actor;
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
        let token = context.myself.local().unwrap().cell.token.clone();
        context.stash_capacity = mailbox.stash_capacity;
        let actor_name = type_name::<A>();
        let started_fut = select! {
            _ = token.cancelled() => {
                Err(anyhow!("actor {} start cancelled", actor_name))
            }
            result = actor.started(&mut context) => {
                result
            }
        };
        if let Err(err) = started_fut {
            error!("actor {} start error {:?}", actor_name, err);
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
            select! {
                biased;
                Some(message) = mailbox.system.recv() => {
                    Self::handle_system(&mut context, &mut actor, message).await;
                    if matches!(context.state, ActorState::CanTerminate) {
                        break;
                    }
                }
                Some(message) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    let name = message.name();
                    if let Err(_) = AssertUnwindSafe(Self::handle_message(&mut context, &mut actor, message, &token)).catch_unwind().await {
                        Self::handle_failure(&mut context, anyhow!("{} handle message {} panic", type_name::<A>(), name));
                    }
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
        if let Err(err) = actor.stopped(&mut context).await {
            error!("actor {} stop error {:?}", actor_name, err);
        }
        mailbox.close();
        context.state = ActorState::Terminated;
        for (name, handle) in context.abort_handles {
            handle.abort();
            debug!("{} abort task: {}", type_name::<A>(), name);
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
                let name = type_name::<A>();
                let myself = context.myself.clone();
                context.handle_invoke_failure(myself, name, logic_error);
            }
            Err(_) => {
                let name = type_name::<A>();
                let myself = context.myself.clone();
                context.handle_invoke_failure(myself, name, anyhow!("{} panic", name));
            }
            _ => {}
        }
    }

    async fn handle_message(context: &mut ActorContext, actor: &mut A, envelope: Envelope, token: &CancellationToken) {
        let Envelope { message, sender } = envelope;
        let name = message.name();
        context.sender = sender;
        select! {
            _ = token.cancelled() => {
                Self::handle_failure(context, anyhow!("{} cancelled", name));
            }
            result = actor.on_recv(context, message) => {
                if let Err(error) = result {
                    Self::handle_failure(context, error);
                }
            }
        }
        context.sender.take();
    }

    fn handle_failure(context: &mut ActorContext, error: Error) {
        let name = type_name::<A>();
        let myself = context.myself.clone();
        context.handle_invoke_failure(myself, name, error);
    }
}